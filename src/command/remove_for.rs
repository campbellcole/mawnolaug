use poise::serenity_prelude::User;

use crate::{command::remove::remove_channel_for, Context};

super::command! {
    true;
    /// Remove the monologue channel for the provided user if one exists
    ///
    /// **Admin only**
    pub async fn remove_for(
        ctx: Context<'_>,
        #[description = "The user whose channel to remove"]
        user: User,
    ) -> Result<()> {
        remove_channel_for(&ctx, &user).await
    }
}
