use poise::CreateReply;

use crate::{random_draw::do_random_draw, Context};

super::command! {
    true;
    pub async fn trigger(ctx: Context<'_>) -> Result<()> {
        let Some(random_draw) = &ctx.data().config.random_draw else {
            ctx.send(CreateReply::default().content("Random draw is not configured").ephemeral(true)).await?;

            return Ok(());
        };

        do_random_draw(random_draw, ctx.data(), ctx.http()).await
    }
}
