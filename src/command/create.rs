use color_eyre::eyre::{OptionExt, Result};
use poise::{
    serenity_prelude::{
        ChannelId, PermissionOverwrite, PermissionOverwriteType, Permissions, User,
    },
    CreateReply,
};
use serde::Serialize;

use crate::Context;

super::command! {
    false;
    /// Create a monologue channel for yourself
    pub async fn create(ctx: Context<'_>) -> Result<()> {
        let user = ctx.author();

        create_channel_for(&ctx, user).await
    }
}

#[derive(Serialize)]
struct CreateChannel {
    name: String,
    parent_id: Option<ChannelId>,
}

pub async fn create_channel_for(ctx: &Context<'_>, user: &User) -> Result<()> {
    let mut state = ctx.data().state.lock().await;

    if state.get_channel(user.id).is_some() {
        ctx.send(
            CreateReply::default()
                .content(format!("Channel already exists for <@{}>", user.id))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

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
                "mawnolaug channel created by <@{}>",
                ctx.author().id,
            )),
        )
        .await?;

    if !ctx.data().config.monologues.allow_anyone {
        let everyone = guild_id.everyone_role();

        let permission = PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(everyone),
        };

        channel.create_permission(ctx.http(), permission).await?;

        let permission = PermissionOverwrite {
            allow: Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user.id),
        };

        channel.create_permission(ctx.http(), permission).await?;
    }

    state.set_channel(user.id, channel.id).await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Created channel: <#{}>", channel.id))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
