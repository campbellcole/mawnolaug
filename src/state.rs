use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use poise::serenity_prelude::{ChannelId, UserId};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    state_file: PathBuf,
    /// Mapping of user IDs to monologue channel IDs
    channels: HashMap<UserId, ChannelId>,
    /// Last trigger time for random draw
    last_trigger: Option<DateTime<Utc>>,
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

    pub async fn save(&self) -> Result<()> {
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

    pub fn should_track(&self, channel_id: ChannelId) -> bool {
        self.channels.values().any(|&id| id == channel_id)
    }

    pub async fn set_channel(&mut self, user_id: UserId, channel_id: ChannelId) -> Result<()> {
        self.channels.insert(user_id, channel_id);

        self.save().await?;

        Ok(())
    }

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
            self.save().await?;
            return Ok(Some(user_id));
        }

        Ok(None)
    }

    pub async fn remove_channel_for(&mut self, user_id: &UserId) -> Result<Option<ChannelId>> {
        let id = self.channels.remove(user_id);

        if id.is_some() {
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
}
