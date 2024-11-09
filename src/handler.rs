use color_eyre::eyre::{Result, WrapErr};
use poise::BoxFuture;
use serenity::all::{ChannelId, Context, FullEvent, GuildChannel, Message, MessageId};

use crate::{
    data::{Data, FrameworkContext},
    utils, STARTUP_TIME,
};

async fn message(ctx: &Context, data: &Data, msg: &Message) -> Result<()> {
    // ignore messages sent before the bot started up. since we always index on
    // startup, we already know about these messages and skipping them prevents
    // duplicates from ending up in the index
    if &*msg.timestamp < STARTUP_TIME.get().unwrap() {
        return Ok(());
    }

    // should_track checks if this channel is a monologue channel and we only
    // care about messages sent to monologue channels
    if !data.state.lock().await.should_track(msg.channel_id) {
        return Ok(());
    }

    let channel_id = msg.channel_id;

    // save the message to the index. this function internally checks if the
    // message is valid and will do nothing if it is not
    data.index
        .lock()
        .await
        .save_message(msg)
        .await
        .wrap_err("failed to save message")?;

    // if autosort is enabled, trigger the channel sorting mechanism
    if data.config.is_autosort_enabled() {
        utils::move_channel_to_top(ctx, data, channel_id)
            .await
            .wrap_err("failed to move channel to top")?;
    }

    Ok(())
}

async fn delete_messages_inner(
    data: &Data,
    channel_id: &ChannelId,
    deleted_message_ids: impl IntoIterator<Item = &MessageId>,
) -> Result<()> {
    // same concept as above just with a bulk delete

    // we only care about monologue channels
    if !data.state.lock().await.should_track(*channel_id) {
        return Ok(());
    }

    for deleted_message_id in deleted_message_ids {
        // remove the message from the index. if we don't do this, there's a
        // chance it'll be randomly drawn and produce an error
        data.index
            .lock()
            .await
            .remove_message(*channel_id, *deleted_message_id)
            .await
            .wrap_err("failed to remove message")?;
    }

    Ok(())
}

async fn channel_delete(data: &Data, channel: &GuildChannel) -> Result<()> {
    let mut state_lock = data.state.lock().await;

    if !state_lock.should_track(channel.id) {
        return Ok(());
    }

    // we need to remove the channel from both state and index to prevent random
    // draws and indexing from failing
    state_lock
        .remove_channel(channel.id)
        .await
        .wrap_err("failed to remove channel")?;

    data.index
        .lock()
        .await
        .remove_channel(channel.id)
        .await
        .wrap_err("failed to remove channel")?;

    Ok(())
}

pub fn event_handler<'a>(
    ctx: &'a Context,
    event: &'a FullEvent,
    _framework_ctx: FrameworkContext<'a>,
    data: &'a Data,
) -> BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        match event {
            FullEvent::Message { new_message } => message(ctx, data, new_message).await?,
            FullEvent::MessageDelete {
                channel_id,
                deleted_message_id,
                guild_id: _,
            } => {
                delete_messages_inner(data, channel_id, std::iter::once(deleted_message_id)).await?
            }
            FullEvent::MessageDeleteBulk {
                channel_id,
                multiple_deleted_messages_ids,
                guild_id: _,
            } => delete_messages_inner(data, channel_id, multiple_deleted_messages_ids).await?,
            FullEvent::ChannelDelete {
                channel,
                messages: _,
            } => channel_delete(data, channel).await?,
            _ => {}
        }

        Ok(())
    })
}
