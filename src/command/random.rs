use color_eyre::eyre::Result;
use poise::{command, serenity_prelude::User, CreateReply};

use crate::{utils, Context};

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

    ctx.send(
        CreateReply::default().content(utils::repost_message(message, "Random message:").await?),
    )
    .await?;

    Ok(())
}
