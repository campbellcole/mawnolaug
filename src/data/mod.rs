use std::sync::Arc;

use color_eyre::eyre::{Report, Result};
use tokio::sync::Mutex;

pub mod config;
pub mod index;
pub mod state;

/// The main data struct that contains the config, state, and index.
#[derive(Debug)]
pub struct DataInner {
    pub config: config::AppConfig,
    pub state: Mutex<state::State>,
    pub index: Mutex<index::Index>,
}

pub async fn load() -> Result<Data> {
    debug!("Loading configuration");
    let config = config::AppConfig::load()?;

    debug!("Loading state");
    let state = state::State::load(&config).await?;

    debug!("Loading index");
    let index = index::Index::load(&config).await?;

    let data = DataInner {
        config,
        state: Mutex::new(state),
        index: Mutex::new(index),
    };

    Ok(Arc::new(data))
}

pub type Data = Arc<DataInner>;
pub type Error = Report;

// a collection of type aliases for various poise types that take both of these
// type parameters
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type Command = poise::Command<Data, Error>;
pub type FrameworkContext<'a> = poise::FrameworkContext<'a, Data, Error>;
pub type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;
