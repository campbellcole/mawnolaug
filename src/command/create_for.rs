use serenity::all::User;

use crate::{command::create::create_channel_for, data::Context};

super::command! {
    true;
    /// Create a monologue channel for the provided user
    ///
    /// **Admin only**
    pub async fn create_for(
        ctx: Context<'_>,
        #[description = "The user for whom to create the channel"]
        user: User,
    ) -> Result<()> {
        create_channel_for(&ctx, &user).await
    }
}
