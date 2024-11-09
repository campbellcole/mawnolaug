use color_eyre::eyre::Result;
use poise::CreateReply;
use serenity::all::User;

use crate::data::Context;

super::command! {
    false;
    /// Remove your monologue channel if one exists
    pub async fn remove(ctx: Context<'_>) -> Result<()> {
        let user = ctx.author();

        remove_channel_for(&ctx, user).await
    }
}

/// Remove the monologue channel for the provided user if one exists
pub async fn remove_channel_for(ctx: &Context<'_>, user: &User) -> Result<()> {
    // if the user has a channel, remove it and return it, otherwise send a
    // reply stating that no channel exists
    let Some(channel_id) = ctx
        .data()
        .state
        .lock()
        .await
        .remove_channel_for(&user.id)
        .await?
    else {
        trace!("no monologue channel exists for {}", user.name);

        ctx.send(
            CreateReply::default()
                .content(format!("No monologue channel exists for <@{}>", user.id))
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    };

    trace!("deleting monologue channel for {}", user.name);
    // delete the channel
    ctx.http()
        .delete_channel(
            channel_id,
            Some(&format!(
                "mawnolaug channel removed by {}",
                ctx.author().name,
            )),
        )
        .await?;

    // remove the channel from the index
    ctx.data()
        .index
        .lock()
        .await
        .remove_channel(channel_id)
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Removed monologue channel for <@{}>", user.id))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
