use color_eyre::eyre::Result;
use poise::{command, serenity_prelude::User};

use crate::{command::create::create_channel_for, Context};

#[command(slash_command)]
pub async fn create_for(ctx: Context<'_>, user: User) -> Result<()> {
    create_channel_for(&ctx, &user).await
}
