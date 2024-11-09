use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
use poise::{Framework, FrameworkOptions};
use random_draw::random_draw_task;
use serenity::all::{ActivityData, ClientBuilder, GatewayIntents};
use tracing_subscriber::prelude::*;

#[macro_use]
extern crate tracing;

pub mod command;
pub mod data;
pub mod error;
pub mod handler;
pub mod random_draw;
pub mod utils;

/// A global lock for the startup time of the bot. Useful for checking if the
/// message event handler is receiving old messages.
pub static STARTUP_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();

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

    let data = data::load().await?;
    let token = data.config.token.clone();

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
            on_error: error::handle_error,
            event_handler: handler::event_handler,
            // we don't use the owner system so just disable it entirely
            initialize_owners: false,
            skip_checks_for_owners: true,
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                trace!("registering commands");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                trace!("running startup index");
                let channels = data.state.lock().await.get_channels();
                data.index.lock().await.index(ctx, channels).await?;

                // start the random draw task
                tokio::task::spawn(random_draw_task(data.clone(), ctx.http.clone()));

                // :eyes:
                ctx.set_activity(Some(ActivityData::watching("you shitpost")));

                Ok(data)
            })
        })
        .build();

    debug!("Creating client");
    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    info!("Starting up");
    client.start().await?;

    Ok(())
}
