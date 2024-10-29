use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
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
            return Ok(Self {
                index_file,
                last_indexed: HashMap::new(),
                messages: HashMap::new(),
            });
        }

        let index = tokio::fs::read_to_string(&index_file).await?;

        let mut index = serde_json::from_str::<Self>(&index)?;

        index.index_file = index_file;

        Ok(index)
    }

    pub async fn save(&self) -> Result<()> {
        let index_json = serde_json::to_string(self)?;

        tokio::fs::write(&self.index_file, index_json).await?;

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

    // Precondition: All messages must be from the same channel.
    fn save_messages(&mut self, channel_id: ChannelId, messages: impl Iterator<Item = Message>) {
        self.messages
            .entry(channel_id)
            .or_default()
            .extend(messages.filter(Self::is_message_valid).map(|msg| msg.id));
    }

    fn save_message_inner(&mut self, message: Message) {
        if !Self::is_message_valid(&message) {
            return;
        }

        self.messages
            .entry(message.channel_id)
            .or_default()
            .push(message.id);
    }

    pub async fn save_message(&mut self, message: Message) -> Result<()> {
        self.save_message_inner(message);

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

        let mut rng = rand::thread_rng();
        let mut random_index = rng.gen_range(0..total_count);

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
        let mut rng = rand::thread_rng();
        let messages = self.messages.get(&channel_id)?;

        messages.choose(&mut rng).copied()
    }

    pub fn random_message_since(&self, timestamp: DateTime<Utc>) -> Option<MessageId> {
        let mut rng = rand::thread_rng();
        let all_messages: Vec<_> = self
            .messages
            .values()
            .flatten()
            .filter(|msg| *msg.created_at() > timestamp)
            .collect();

        all_messages.choose(&mut rng).map(|v| **v)
    }

    pub async fn remove_message(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        if let Some(messages) = self.messages.get_mut(&channel_id) {
            messages.retain(|&id| id != message_id);
            self.save().await?;
        }

        Ok(())
    }

    pub async fn remove_channel(&mut self, channel_id: ChannelId) -> Result<()> {
        self.messages.remove(&channel_id);
        self.save().await?;
        Ok(())
    }

    pub async fn index(&mut self, ctx: &Context, channels: Vec<ChannelId>) -> Result<()> {
        for channel_id in channels {
            trace!("indexing channel {:?}", channel_id);
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

                self.save_messages(channel_id, messages.into_iter());

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
