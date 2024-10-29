use color_eyre::eyre::Result;
use poise::{command, serenity_prelude::User};

use crate::{command::remove::remove_channel_for, Context};

#[command(slash_command)]
pub async fn remove_for(ctx: Context<'_>, user: User) -> Result<()> {
    remove_channel_for(&ctx, &user).await
}
