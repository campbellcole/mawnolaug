use poise::serenity_prelude::User;

use crate::{command::remove::remove_channel_for, Context};

super::command! {
    true;
    pub async fn remove_for(ctx: Context<'_>, user: User) -> Result<()> {
        remove_channel_for(&ctx, &user).await
    }
}
