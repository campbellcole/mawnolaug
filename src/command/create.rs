use color_eyre::eyre::{OptionExt, Result};
use poise::{
    command,
    serenity_prelude::{ChannelId, User},
    CreateReply,
};
use serde::Serialize;

use crate::Context;

#[command(slash_command)]
pub async fn create(ctx: Context<'_>) -> Result<()> {
    let user = ctx.author();

    create_channel_for(&ctx, user).await
}

#[derive(Serialize)]
struct CreateChannel {
    name: String,
    parent_id: Option<ChannelId>,
}

pub async fn create_channel_for(ctx: &Context<'_>, user: &User) -> Result<()> {
    let user_name = user.global_name.as_ref().unwrap_or(&user.name);

    let channel_name = user_name.replace(' ', "_");

    let channel = CreateChannel {
        name: channel_name,
        parent_id: ctx.data().config.monologues.category_id,
    };

    let guild_id = ctx.guild_id().ok_or_eyre("Not in a guild")?;

    let channel = ctx
        .http()
        .create_channel(
            guild_id,
            &channel,
            Some(&format!(
                "mawnolaug channel created by {}",
                ctx.author().name,
            )),
        )
        .await?;

    ctx.data()
        .state
        .lock()
        .await
        .set_channel(user.id, channel.id)
        .await?;

    ctx.send(CreateReply::default().content(format!("Created channel: <#{}>", channel.id)))
        .await?;

    Ok(())
}
