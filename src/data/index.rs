use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, WrapErr};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Context, GetMessages, Message, MessageId, MessageType};

use crate::data::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    #[serde(skip)]
    index_file: PathBuf,
    last_indexed: HashMap<ChannelId, MessageId>,
    messages: HashMap<ChannelId, Vec<MessageId>>,
}

impl Index {
    pub async fn load(config: &AppConfig) -> Result<Self> {
        let index_file = config.state_dir.join("index.json");

        if !index_file.exists() {
            trace!("creating a new index file at {:?}", index_file);
            return Ok(Self {
                index_file,
                last_indexed: HashMap::new(),
                messages: HashMap::new(),
            });
        }

        let index = tokio::fs::read_to_string(&index_file)
            .await
            .wrap_err("failed to read index file")?;

        let mut index =
            serde_json::from_str::<Self>(&index).wrap_err("failed to parse index file")?;

        index.index_file = index_file;

        Ok(index)
    }

    async fn save(&self) -> Result<()> {
        trace!("saving index");

        let index_json = serde_json::to_string(self).wrap_err("failed to serialize index")?;

        tokio::fs::write(&self.index_file, index_json)
            .await
            .wrap_err("failed to write serialized index")?;

        Ok(())
    }

    pub fn last_indexed(&self, channel_id: &ChannelId) -> Option<MessageId> {
        self.last_indexed.get(channel_id).copied()
    }

    fn just_indexed_inner(&mut self, channel_id: ChannelId, message_id: MessageId) {
        self.last_indexed.insert(channel_id, message_id);
    }

    pub async fn just_indexed(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        self.just_indexed_inner(channel_id, message_id);

        self.save().await?;

        Ok(())
    }

    fn is_message_valid(message: &Message) -> bool {
        !message.author.bot
            // && message.poll.is_none() need to wait for serenity 0.12.3 with
            // fix for #2892
            && matches!(
                message.kind,
                MessageType::Regular | MessageType::InlineReply
            )
    }

    /// Precondition: All messages must be from the same channel.
    ///
    /// Caller is responsible for ensuring `last_indexed` is updated correctly.
    fn extend_messages(&mut self, channel_id: ChannelId, messages: impl Iterator<Item = Message>) {
        self.messages
            .entry(channel_id)
            .or_default()
            .extend(messages.filter(Self::is_message_valid).map(|msg| msg.id));
    }

    /// Save a message to the index if it is valid.
    pub async fn save_message(&mut self, message: &Message) -> Result<()> {
        if !Self::is_message_valid(message) {
            return Ok(());
        }

        self.messages
            .entry(message.channel_id)
            .or_default()
            .push(message.id);

        self.just_indexed_inner(message.channel_id, message.id);

        self.save().await?;

        Ok(())
    }

    pub fn get_messages(&self, channel_id: &ChannelId) -> Option<Vec<MessageId>> {
        self.messages.get(channel_id).cloned()
    }

    /// Randomly draw a message from all indexed messages.
    ///
    /// This deliberately does not maintain an equal distribution between
    /// channels; if one channel has many more messages than another, it will be
    /// more likely to be selected.
    pub fn random_message(&self) -> Option<(ChannelId, MessageId)> {
        // this works by treating all messages as a single list and picking a
        // random index into that quasi-list. we then iterate over each
        // sub-list, subtracting its length from the index until the index falls
        // within a sub-list

        let total_count = self.messages.values().map(|v| v.len()).sum::<usize>();

        if total_count == 0 {
            return None;
        }

        let mut random_index = rand::thread_rng().gen_range(0..total_count);

        for (key, vec) in &self.messages {
            if random_index < vec.len() {
                return Some((*key, vec[random_index]));
            } else {
                random_index -= vec.len();
            }
        }

        // not possible
        None
    }

    /// Randomly draw a message from all indexed messages in the given channel.
    ///
    /// Since the caller already has the channel ID, we just return the message
    /// ID.
    // NOTE TO PROGRAMMER: there is no point in taking a reference to any ID
    // types because they're all `NonZeroU64`s which are the same size as a
    // reference
    pub fn random_message_from(&self, channel_id: ChannelId) -> Option<MessageId> {
        let messages = self.messages.get(&channel_id)?;

        messages.choose(&mut rand::thread_rng()).copied()
    }

    /// Randomly draw a message from all indexed messages that were created
    /// after the given timestamp.
    ///
    /// This deliberately does not maintain an equal distribution between
    /// channels; if one channel has many more messages than another, it will be
    /// more likely to be selected.
    pub fn random_message_since(&self, timestamp: DateTime<Utc>) -> Option<(ChannelId, MessageId)> {
        // we need to maintain the association between channel and message id so
        // we can't just flatten the hashmap. instead we flat_map each
        // `(channel, message_ids)` pair into an iterator of `(channel,
        // message_id)` pairs. since Discord message ids contain the timestamp,
        // we do the filtering at the same time as mapping to avoid iterating
        // over the entire list of messages twice
        let all_messages: Vec<_> = self
            .messages
            .iter()
            .flat_map(|(channel_id, message_ids)| {
                message_ids.iter().filter_map(|&message_id| {
                    if *message_id.created_at() >= timestamp {
                        Some((*channel_id, message_id))
                    } else {
                        None
                    }
                })
            })
            .collect();

        all_messages.choose(&mut rand::thread_rng()).copied()
    }

    /// Remove a message from the index.
    pub async fn remove_message(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        let Some(messages) = self.messages.get_mut(&channel_id) else {
            return Ok(());
        };

        // remove the message
        messages.retain(|&id| id != message_id);

        // if the message was the last indexed message, we need to replace it
        // with the new latest message (or delete it if that was the only msg)
        if self.last_indexed(&channel_id) == Some(message_id) {
            // SAFETY: we know this must exist, we just can't use the `messages`
            // borrow because we need an immutable borrow for `last_indexed`
            match self.messages.get_mut(&channel_id).unwrap().last().copied() {
                Some(msg) => {
                    self.just_indexed_inner(channel_id, msg);
                }
                None => {
                    self.last_indexed.remove(&channel_id);
                }
            }
        }

        self.save().await?;

        Ok(())
    }

    /// Remove an entire channel from the index.
    ///
    /// Caller is responsible for removing it from the state as well.
    pub async fn remove_channel(&mut self, channel_id: ChannelId) -> Result<()> {
        self.messages.remove(&channel_id);
        self.last_indexed.remove(&channel_id);

        self.save().await?;

        Ok(())
    }

    /// Index all messages in the given channels.
    ///
    ///
    pub async fn index(&mut self, ctx: &Context, channels: Vec<ChannelId>) -> Result<()> {
        for channel_id in channels {
            debug!("indexing channel {:?}", channel_id);
            let mut current_message = self.last_indexed(&channel_id);
            let mut latest_message = None;
            // if we already have a current message, we should go forward in
            // time
            let forward = current_message.is_some();
            trace!(?forward, ?current_message);

            loop {
                let request = match current_message {
                    Some(current) => {
                        if forward {
                            GetMessages::new().after(current)
                        } else {
                            GetMessages::new().before(current)
                        }
                    }
                    None => GetMessages::new(),
                }
                .limit(100);
                trace!(?request);

                let messages = channel_id.messages(&ctx, request).await?;
                trace!(?messages);

                // SAFETY: all unwraps below this statement are safe because we
                // check this
                if messages.is_empty() {
                    trace!(?current_message, "no more messages");
                    break;
                }

                // these lines contain some magic that should be explained: the
                // messages are always returned with the newest message first.
                // however, if we are indexing for the first time, we are going
                // to go backwards but still need the newest message so that the
                // next index starts from the correct message. its important to
                // note that this function will not behave correctly if it is
                // somehow instructed to iterate backwards starting from a
                // message that is not the latest message; in that case, it will
                // require another call to `index` to finish by going forwards
                let next_message_id = if forward {
                    // if we are going forwards, the first message is the
                    // current newest message, so next_message_id and
                    // latest_message will always be equal
                    latest_message = Some(messages.first().unwrap().id);
                    latest_message
                } else {
                    // if we are going backwards, we need to set the latest
                    // message only once on the first iteration, then use the
                    // last message as the next message id so iterating
                    // progresses backwards
                    if latest_message.is_none() {
                        latest_message = Some(messages.first().unwrap().id);
                    }
                    Some(messages.last().unwrap().id)
                };
                trace!(?next_message_id);

                self.extend_messages(channel_id, messages.into_iter());

                current_message = next_message_id;
            }

            if let Some(latest_message) = latest_message {
                self.just_indexed_inner(channel_id, latest_message);
            }
        }

        self.save().await?;

        Ok(())
    }
}
