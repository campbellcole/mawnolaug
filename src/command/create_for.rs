use poise::serenity_prelude::User;

use crate::{command::create::create_channel_for, Context};

super::command! {
    true;
    pub async fn create_for(ctx: Context<'_>, user: User) -> Result<()> {
        create_channel_for(&ctx, &user).await
    }
}
