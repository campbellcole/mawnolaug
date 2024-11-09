use poise::CreateReply;

use crate::{data::Context, random_draw::do_random_draw};

super::command! {
    true;
    /// Trigger a random draw
    ///
    /// **Admin only**
    pub async fn trigger(ctx: Context<'_>) -> Result<()> {
        let Some(random_draw) = &ctx.data().config.random_draw else {
            trace!("random draw is not configured");

            ctx.send(CreateReply::default().content("Random draw is not configured").ephemeral(true)).await?;

            return Ok(());
        };

        do_random_draw(random_draw, ctx.data(), ctx.http()).await?;

        ctx.send(CreateReply::default().content("Random draw triggered").ephemeral(true)).await?;

        Ok(())
    }
}
