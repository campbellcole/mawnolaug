use std::{ops::Deref, path::PathBuf};

use color_eyre::eyre::Result;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use poise::serenity_prelude::{ChannelId, Permissions};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
pub struct AdminPermissions(pub Permissions);

impl Default for AdminPermissions {
    fn default() -> Self {
        AdminPermissions(Permissions::ADMINISTRATOR)
    }
}

impl Deref for AdminPermissions {
    type Target = Permissions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// The Discord token used to authenticate the bot
    pub token: String,
    /// The permissions required for usage of the admin commands
    ///
    /// See [Discord permissions](https://discord.com/developers/docs/topics/permissions)
    #[serde(default)]
    pub admin_permissions: AdminPermissions,
    /// The data folder to store state
    pub state_dir: PathBuf,
    /// The configuration for the random draw feature
    #[serde(default)]
    pub random_draw: Option<RandomDrawConfig>,
    /// Configuration for monologue channels
    pub monologues: MonologuesConfig,
}

#[derive(Debug, Deserialize)]
pub struct RandomDrawConfig {
    /// The channel ID where the bot will send messages
    pub channel_id: ChannelId,
    /// A cron schedule for when to trigger the random draws
    pub schedule: Schedule,
    /// The timezone to use for the cron schedule
    #[serde(default)]
    pub timezone: Option<chrono_tz::Tz>,
    /// A list of messages to prefix each random draw with
    #[serde(default)]
    pub messages: Option<Vec<String>>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Schedule(#[serde_as(as = "DisplayFromStr")] cron::Schedule);

impl Deref for Schedule {
    type Target = cron::Schedule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct MonologuesConfig {
    /// The category ID for the monologue channels
    #[serde(default)]
    pub category_id: Option<ChannelId>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_file = match std::env::var("MAWNO_CONFIG") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                let mut path = std::env::current_dir()?;
                path.push("mawnolaug.toml");
                path
            }
        };
        trace!("Loading configuration from {:?}", config_file);

        let config = Figment::new()
            .merge(Toml::file(config_file))
            .merge(Env::prefixed("MAWNO_").global().split('_'))
            .extract::<AppConfig>()?;

        Ok(config)
    }
}
