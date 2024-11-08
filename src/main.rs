use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Report, Result};
use handler::Handler;
use poise::{
    serenity_prelude::{prelude::TypeMapKey, ActivityData, ClientBuilder, GatewayIntents},
    BoxFuture, CreateReply, Framework, FrameworkError, FrameworkOptions,
};
use random_draw::random_draw_task;
use tokio::sync::Mutex;
use tracing_subscriber::prelude::*;

#[macro_use]
extern crate tracing;

pub mod command;
pub mod config;
pub mod handler;
pub mod index;
pub mod random_draw;
pub mod state;
pub mod utils;

#[derive(Debug)]
pub struct Data {
    pub config: config::AppConfig,
    pub state: Mutex<state::State>,
    pub index: Mutex<index::Index>,
}

#[derive(Debug)]
pub struct DataHolder(pub Arc<Data>);

impl DataHolder {
    pub async fn get(ctx: &poise::serenity_prelude::Context) -> Arc<Data> {
        ctx.data
            .read()
            .await
            .get::<DataHolderKey>()
            .unwrap()
            .0
            .clone()
    }
}

pub struct DataHolderKey;

impl TypeMapKey for DataHolderKey {
    type Value = DataHolder;
}

impl Deref for DataHolder {
    type Target = Data;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, Report>;

pub static STARTUP_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();

async fn handle_error_inner(err: FrameworkError<'_, Arc<Data>, Report>) -> Result<()> {
    match err {
        FrameworkError::Setup { error, .. } => {
            error!("setup error: {:?}", error);
        }
        FrameworkError::Command { error, ctx, .. } => {
            error!("command error: {:?}", error);
            ctx.send(
                CreateReply::default()
                    .content(format!("Error: {:#}", error))
                    .ephemeral(true),
            )
            .await?;
        }
        FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            error!("missing bot permissions: {}", missing_permissions);
            ctx.send(
                CreateReply::default()
                    .content(format!("Missing bot permissions: {}", missing_permissions))
                    .ephemeral(true),
            )
            .await?;
        }
        _ => {
            poise::builtins::on_error(err).await?;
        }
    }

    Ok(())
}

pub fn handle_error(err: FrameworkError<'_, Arc<Data>, Report>) -> BoxFuture<()> {
    Box::pin(async move {
        if let Err(err) = handle_error_inner(err).await {
            error!("error handling error: {:?}", err);
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_error::ErrorLayer::default())
        .init();

    color_eyre::install()?;

    STARTUP_TIME.set(Utc::now()).unwrap();

    debug!("Loading configuration");
    let config = config::AppConfig::load()?;
    let token = config.token.clone();

    debug!("Loading state");
    let state = state::State::load(&config).await?;

    debug!("Loading index");
    let index = index::Index::load(&config).await?;

    let data = Data {
        config,
        state: Mutex::new(state),
        index: Mutex::new(index),
    };
    let data = Arc::new(data);
    let data_holder = DataHolder(Arc::clone(&data));

    let intents =
        // allow creation/deletion of monologue channels
        GatewayIntents::GUILDS
        // allow deleting messages of users posting outside their channel
        | GatewayIntents::GUILD_MESSAGES
        // allow reading messages in monologue channels
        | GatewayIntents::MESSAGE_CONTENT;

    debug!(?intents, "Starting bot");
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: command::commands(&data.config),
            initialize_owners: false,
            skip_checks_for_owners: true,
            on_error: handle_error,
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                trace!("registering commands");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let channels = data.state.lock().await.get_channels();

                trace!("running startup index");
                data.index.lock().await.index(ctx, channels).await?;

                tokio::task::spawn(random_draw_task(data.clone(), ctx.http.clone()));

                ctx.set_activity(Some(ActivityData::watching("you shitpost")));

                Ok(data)
            })
        })
        .build();

    debug!("Creating client");
    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(Handler)
        .type_map_insert::<DataHolderKey>(data_holder)
        .await?;

    info!("Starting up");
    client.start().await?;

    Ok(())
}
