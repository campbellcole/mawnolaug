use color_eyre::eyre::Result;
use poise::{command, CreateReply};
use serenity::all::User;

use crate::{
    data::{config::AppConfig, Command, Context},
    utils,
};

// the `command!` macro somehow alters the `Option<User>` in a way that breaks
// the `poise::command` macro. Instead of recognizing the argument is optional,
// it assumes `Option<User>` is a distinct type and tries to serialize the
// entire type instead of extracting the `User` type and setting the argument to
// be `required: false`
//
// see https://github.com/serenity-rs/poise/issues/317
pub fn command(_config: &AppConfig) -> Command {
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
    crate::command::__trace_cmd!(ctx ctx, "random");

    let (channel_id, message_id) = if let Some(user) = user {
        // if the user is specified, get a random message from their channel

        // get their channel if it exists
        let Some(user_channel) = ctx.data().state.lock().await.get_channel(user.id) else {
            trace!("no channel exists for {}", user.name);

            ctx.send(
                CreateReply::default()
                    .content(format!("No channel exists for <@{}>", user.id))
                    .ephemeral(true),
            )
            .await?;

            return Ok(());
        };

        // get a random message from their channel if any exist
        let Some(random) = ctx
            .data()
            .index
            .lock()
            .await
            .random_message_from(user_channel)
        else {
            trace!("channel for {} contains no messages", user.name);

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
        // if the user is not specified, get a random message from any channel,
        // if any exist
        let Some((channel_id, message_id)) = ctx.data().index.lock().await.random_message() else {
            trace!("no messages in any channel");
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

    trace!(?channel_id, ?message_id, "fetching message");
    // fetch the message object
    let message = channel_id.message(ctx, message_id).await?;

    // reply with the formatted message content
    ctx.send(CreateReply::default().content(utils::format_repost_content(message, None)))
        .await?;

    Ok(())
}
