use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, WrapErr};
use poise::serenity_prelude::{ChannelId, Context, GetMessages, Message, MessageId, MessageType};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

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

    pub async fn save(&self) -> Result<()> {
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

    pub fn is_message_valid(message: &Message) -> bool {
        !message.author.bot
            && message.poll.is_none()
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

    pub async fn save_message(&mut self, message: Message) -> Result<()> {
        if !Self::is_message_valid(&message) {
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
    /// This deliberately does not maintain an equal distribution between channels; if one channel
    /// has many more messages than another, it will be more likely to be selected.
    pub fn random_message(&self) -> Option<(ChannelId, MessageId)> {
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

    pub fn random_message_from(&self, channel_id: ChannelId) -> Option<MessageId> {
        let messages = self.messages.get(&channel_id)?;

        messages.choose(&mut rand::thread_rng()).copied()
    }

    pub fn random_message_since(&self, timestamp: DateTime<Utc>) -> Option<(ChannelId, MessageId)> {
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

    pub async fn remove_message(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        if let Some(messages) = self.messages.get_mut(&channel_id) {
            messages.retain(|&id| id != message_id);

            if self.last_indexed(&channel_id) == Some(message_id) {
                // SAFETY: we know this must exist, we just can't use the `messages` borrow because
                // we need an immutable borrow for `last_indexed`
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
        }

        Ok(())
    }

    pub async fn remove_channel(&mut self, channel_id: ChannelId) -> Result<()> {
        self.messages.remove(&channel_id);
        self.last_indexed.remove(&channel_id);

        self.save().await?;

        Ok(())
    }

    pub async fn index(&mut self, ctx: &Context, channels: Vec<ChannelId>) -> Result<()> {
        for channel_id in channels {
            debug!("indexing channel {:?}", channel_id);
            let mut current_message = self.last_indexed(&channel_id);
            let mut latest_message = None;
            // if we already have a current message, we should go forward in time
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

                if messages.is_empty() {
                    trace!(?current_message, "no more messages");
                    break;
                }

                let next_message_id = if forward {
                    latest_message = Some(messages.first().unwrap().id);
                    latest_message
                } else {
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
