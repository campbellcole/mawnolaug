use color_eyre::eyre::{OptionExt, Result};
use poise::CreateReply;
use serde::Serialize;
use serenity::all::{ChannelId, PermissionOverwrite, PermissionOverwriteType, Permissions, User};

use crate::{data::Context, utils};

super::command! {
    false;
    /// Create a monologue channel for yourself
    pub async fn create(ctx: Context<'_>) -> Result<()> {
        let user = ctx.author();

        create_channel_for(&ctx, user).await
    }
}

#[derive(Debug, Serialize)]
struct CreateChannel {
    name: String,
    parent_id: Option<ChannelId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<u16>,
}

/// Create a monologue channel for the provided user
pub async fn create_channel_for(ctx: &Context<'_>, user: &User) -> Result<()> {
    let config = &ctx.data().config;
    let mut state = ctx.data().state.lock().await;

    // if the user has a channel already, tell the user and exit
    if state.get_channel(user.id).is_some() {
        trace!("channel already exists for {}", user.name);
        ctx.send(
            CreateReply::default()
                .content(format!("Channel already exists for <@{}>", user.id))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().ok_or_eyre("Not in a guild")?;

    let user_name = user.global_name.as_ref().unwrap_or(&user.name);

    let channel_name = user_name.replace(' ', "_");

    // if channel sorting is enabled, create the channel with the next position
    // immediately instead of moving it to the top after creation
    let position = if config.is_autosort_enabled() {
        let next = utils::checked_next_position(ctx.serenity_context(), None, &mut state).await?;

        Some(next)
    } else {
        None
    };

    let channel = CreateChannel {
        name: channel_name,
        parent_id: ctx.data().config.monologues.category_id,
        position,
    };
    trace!(?channel, "creating channel");

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

    // set up permissions if configured to do so
    if !ctx.data().config.monologues.allow_anyone {
        trace!("setting up permissions");

        // forbid @everyone from sending messages
        let everyone = guild_id.everyone_role();

        let permission = PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(everyone),
        };

        channel.create_permission(ctx.http(), permission).await?;

        // allow the user to send messages
        let permission = PermissionOverwrite {
            allow: Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user.id),
        };

        channel.create_permission(ctx.http(), permission).await?;
    }

    // associate the channel with the user
    state.set_channel(user.id, channel.id).await?;

    if let Some(position) = position {
        state.set_channel_position(channel.id, position).await?;
    }

    ctx.send(
        CreateReply::default()
            .content(format!("Created channel: <#{}>", channel.id))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
