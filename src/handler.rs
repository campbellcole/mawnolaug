use poise::serenity_prelude::{
    async_trait, ChannelId, Context, EventHandler, GuildChannel, GuildId, Message, MessageId,
};

use crate::{utils, DataHolder, STARTUP_TIME};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if &*msg.timestamp < STARTUP_TIME.get().unwrap() {
            return;
        }

        let state = DataHolder::get(&ctx).await;

        if !state.state.lock().await.should_track(msg.channel_id) {
            return;
        }

        let channel_id = msg.channel_id;

        if let Err(err) = state.index.lock().await.save_message(msg).await {
            error!("failed to save message: {:?}", err);
        };

        if let Err(err) = utils::move_channel_to_top(ctx, channel_id).await {
            error!("failed to move channel to top: {:?}", err);
        }
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        let state = DataHolder::get(&ctx).await;

        if !state.state.lock().await.should_track(channel_id) {
            return;
        }

        if let Err(err) = state
            .index
            .lock()
            .await
            .remove_message(channel_id, deleted_message_id)
            .await
        {
            error!("failed to remove message: {:?}", err);
        };
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        let state = DataHolder::get(&ctx).await;

        if !state.state.lock().await.should_track(channel_id) {
            return;
        }

        for deleted_message_id in deleted_message_ids {
            if let Err(err) = state
                .index
                .lock()
                .await
                .remove_message(channel_id, deleted_message_id)
                .await
            {
                error!("failed to remove message: {:?}", err);
            };
        }
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        channel: GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        let state = DataHolder::get(&ctx).await;

        let mut state_lock = state.state.lock().await;

        if !state_lock.should_track(channel.id) {
            return;
        }

        if let Err(err) = state_lock.remove_channel(channel.id).await {
            error!("failed to remove channel: {:?}", err);
        }

        if let Err(err) = state.index.lock().await.remove_channel(channel.id).await {
            error!("failed to remove channel: {:?}", err);
        };
    }
}
