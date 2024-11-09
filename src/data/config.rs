use std::{ops::Deref, path::PathBuf, str::FromStr};

use chrono_tz::Tz;
use color_eyre::eyre::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serenity::all::{ChannelId, Permissions};
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
    #[serde(default)]
    pub monologues: MonologuesConfig,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
pub struct Timezone(chrono_tz::Tz);

impl Default for Timezone {
    fn default() -> Self {
        match try_read_timezone() {
            Ok(tz) => Timezone(tz),
            Err(err) => {
                error!("failed to read timezone, falling back to UTC: {}", err);
                Timezone(chrono_tz::UTC)
            }
        }
    }
}

fn try_read_timezone() -> Result<Tz> {
    let iana = iana_time_zone::get_timezone().wrap_err("failed to get OS timezone")?;

    Tz::from_str(&iana).wrap_err("invalid timezone specified")
}

impl Deref for Timezone {
    type Target = chrono_tz::Tz;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct RandomDrawConfig {
    /// The channel ID where the bot will send messages
    pub channel_id: ChannelId,
    /// A cron schedule for when to trigger the random draws
    pub schedule: Schedule,
    /// A list of messages to prefix each random draw with
    #[serde(default)]
    pub messages: Vec<String>,
    /// The timezone to use when formatting timestamps and for the random draw (if enabled)
    #[serde(default)]
    pub timezone: Timezone,
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

#[derive(Debug, Default, Deserialize)]
pub struct MonologuesConfig {
    /// The category ID for the monologue channels
    #[serde(default)]
    pub category_id: Option<ChannelId>,
    /// Whether or not to skip setting up permissions for the monologue channels
    #[serde(default)]
    pub allow_anyone: bool,
    /// Whether or not to disable auto-sorting of monologue channels based on activity
    #[serde(default)]
    pub disable_sorting: bool,
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

    pub fn is_autosort_enabled(&self) -> bool {
        self.monologues.category_id.is_some() && !self.monologues.disable_sorting
    }
}
