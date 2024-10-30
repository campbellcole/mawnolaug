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

#[command(slash_command)]
pub async fn random(ctx: Context<'_>, user: Option<User>) -> Result<()> {
    let message = if let Some(user) = user {
        let Some(user_channel) = ctx.data().state.lock().await.get_channel(user.id) else {
            ctx.send(
                CreateReply::default().content(format!("No channel exists for {}", user.name)),
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
                    .content(format!("Channel for {} contains no messages", user.name)),
            )
            .await?;

            return Ok(());
        };

        ctx.http().get_message(user_channel, random).await?
    } else {
        let Some((channel_id, message_id)) = ctx.data().index.lock().await.random_message() else {
            ctx.send(CreateReply::default().content("No messages in any channel"))
                .await?;
            return Ok(());
        };

        ctx.http().get_message(channel_id, message_id).await?
    };

    ctx.send(CreateReply::default().content(utils::format_repost_content(message, None::<&str>)))
        .await?;

    Ok(())
}
