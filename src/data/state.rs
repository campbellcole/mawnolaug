use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, UserId};

use crate::data::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    state_file: PathBuf,
    /// Mapping of user IDs to monologue channel IDs
    channels: HashMap<UserId, ChannelId>,
    /// Last trigger time for random draw
    last_trigger: Option<DateTime<Utc>>,
    /// A cache of the order of channels in the monologue category
    ///
    /// This is used to prevent fetching every channel in the category just to
    /// get their positions. The positions are used to order the channels based
    /// on which has been used most recently.
    #[serde(default)]
    channel_positions: HashMap<ChannelId, u16>,
}

impl State {
    pub async fn load(config: &AppConfig) -> Result<Self> {
        let state_file = config.state_dir.join("state.json");

        if !state_file.exists() {
            trace!("creating a new state file at {:?}", state_file);
            return Ok(Self {
                state_file,
                channels: HashMap::new(),
                last_trigger: None,
                channel_positions: HashMap::new(),
            });
        }

        let state = tokio::fs::read_to_string(&state_file)
            .await
            .wrap_err("failed to read state file")?;

        let mut state =
            serde_json::from_str::<Self>(&state).wrap_err("failed to parse state file")?;

        state.state_file = state_file;

        Ok(state)
    }

    async fn save(&self) -> Result<()> {
        trace!("saving state");

        let state_json = serde_json::to_string(self).wrap_err("failed to serialize state")?;

        tokio::fs::write(&self.state_file, state_json)
            .await
            .wrap_err("failed to write serialized state")?;

        Ok(())
    }

    pub fn get_channels(&self) -> Vec<ChannelId> {
        self.channels.values().copied().collect()
    }

    pub fn get_channel(&self, user_id: UserId) -> Option<ChannelId> {
        self.channels.get(&user_id).copied()
    }

    /// Check if a channel ID is a monologue channel
    pub fn should_track(&self, channel_id: ChannelId) -> bool {
        self.channels.values().any(|&id| id == channel_id)
    }

    /// Set the channel for a user ID
    ///
    /// Does not set the channel position. Caller must ensure a position is set
    /// after creation if necessary.
    pub async fn set_channel(&mut self, user_id: UserId, channel_id: ChannelId) -> Result<()> {
        self.channels.insert(user_id, channel_id);

        self.save().await?;

        Ok(())
    }

    /// Remove the channel for a user ID
    ///
    /// Automatically removes its channel position as well
    pub async fn remove_channel(&mut self, channel_id: ChannelId) -> Result<Option<UserId>> {
        let user_id = self.channels.iter().find_map(|(user_id, id)| {
            if *id == channel_id {
                Some(*user_id)
            } else {
                None
            }
        });

        if let Some(user_id) = user_id {
            self.channels.remove(&user_id);

            self.channel_positions.remove(&channel_id);

            self.save().await?;

            return Ok(Some(user_id));
        }

        Ok(None)
    }

    /// Removes the channel for a user ID
    ///
    /// Automatically removes its channel position as well
    pub async fn remove_channel_for(&mut self, user_id: &UserId) -> Result<Option<ChannelId>> {
        let id = self.channels.remove(user_id);

        if let Some(id) = id {
            self.channel_positions.remove(&id);

            self.save().await?;
        }

        Ok(id)
    }

    pub fn last_trigger(&self) -> Option<DateTime<Utc>> {
        self.last_trigger
    }

    pub async fn just_triggered(&mut self) -> Result<()> {
        self.last_trigger = Some(Utc::now());

        self.save().await?;

        Ok(())
    }

    pub fn channel_position(&self, channel_id: ChannelId) -> Option<u16> {
        self.channel_positions.get(&channel_id).copied()
    }

    pub async fn set_channel_position(
        &mut self,
        channel_id: ChannelId,
        position: u16,
    ) -> Result<()> {
        self.channel_positions.insert(channel_id, position);

        self.save().await?;

        Ok(())
    }

    /// Get the next position to use for a channel. The order of channels is
    /// descending, so the next position will be the lowest number in the map
    /// minus 1. If this returns zero, it is time to move all the channels back
    /// to u16::MAX and start over.
    pub fn next_position(&self) -> u16 {
        // we use unwrap or default so if there are no entries, we return zero
        // which will trigger the reset
        self.channel_positions
            .values()
            .copied()
            .min()
            .map(|pos| pos - 1)
            .unwrap_or_default()
    }
}
