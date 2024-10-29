use color_eyre::eyre::Result;
use poise::{command, serenity_prelude::User, CreateReply};

use crate::Context;

#[command(slash_command)]
pub async fn remove(ctx: Context<'_>) -> Result<()> {
    let user = ctx.author();

    remove_channel_for(&ctx, user).await
}

pub async fn remove_channel_for(ctx: &Context<'_>, user: &User) -> Result<()> {
    let Some(channel_id) = ctx
        .data()
        .state
        .lock()
        .await
        .remove_channel_for(&user.id)
        .await?
    else {
        ctx.send(
            CreateReply::default()
                .content(format!("No monologue channel exists for {}", user.name)),
        )
        .await?;
        return Ok(());
    };

    ctx.http()
        .delete_channel(
            channel_id,
            Some(&format!(
                "mawnolaug channel removed by {}",
                ctx.author().name,
            )),
        )
        .await?;

    ctx.data()
        .state
        .lock()
        .await
        .remove_channel_for(&user.id)
        .await?;

    ctx.send(
        CreateReply::default().content(format!("Removed monologue channel for {}", user.name)),
    )
    .await?;

    Ok(())
}
