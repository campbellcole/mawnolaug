use std::sync::Arc;

use color_eyre::eyre::{Report, Result};
use poise::{command, serenity_prelude::User, Command, CreateReply};

use crate::{config::AppConfig, utils, Context, Data};

// the `command!` macro somehow alters the `Option<User>` in a way that breaks the `poise::command`
// macro. Instead of recognizing the argument is optional, it assumes `Option<User>` is a distinct
// type and tries to serialize the entire type instead of extracting the `User` type and setting the
// argument to be `required: false`
//
// see https://github.com/serenity-rs/poise/issues/317
pub fn command(_config: &AppConfig) -> Command<Arc<Data>, Report> {
    random()
}

/// Get a random message from a random channel or a specific user's channel
#[command(slash_command, guild_only)]
pub async fn random(
    ctx: Context<'_>,
    #[description = "The user whose monologue channel the message will be drawn from"] user: Option<
        User,
    >,
) -> Result<()> {
    let (channel_id, message_id) = if let Some(user) = user {
        let Some(user_channel) = ctx.data().state.lock().await.get_channel(user.id) else {
            ctx.send(
                CreateReply::default()
                    .content(format!("No channel exists for <@{}>", user.id))
                    .ephemeral(true),
            )
            .await?;

            return Ok(());
        };

        let Some(random) = ctx
            .data()
            .index
            .lock()
            .await
            .random_message_from(user_channel)
        else {
            ctx.send(
                CreateReply::default()
                    .content(format!("Channel for <@{}> contains no messages", user.id))
                    .ephemeral(true),
            )
            .await?;

            return Ok(());
        };

        (user_channel, random)
    } else {
        let Some((channel_id, message_id)) = ctx.data().index.lock().await.random_message() else {
            ctx.send(
                CreateReply::default()
                    .content("No messages in any channel")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        };

        (channel_id, message_id)
    };

    let message = channel_id.message(ctx, message_id).await?;

    ctx.send(CreateReply::default().content(utils::format_repost_content(
        &ctx.data().config,
        message,
        None,
    )))
    .await?;

    Ok(())
}
