use std::sync::Arc;

use chrono::{Duration, Utc};
use color_eyre::eyre::{Context, Result};
use rand::seq::SliceRandom;
use serenity::all::{CreateMessage, Http};

use crate::{
    data::{config::RandomDrawConfig, Data},
    utils,
};

pub async fn random_draw_task(data: Data, http: Arc<Http>) {
    let Some(random_draw) = &data.config.random_draw else {
        debug!("random draw is disabled");
        return;
    };

    debug!("starting random draw task");

    let tz = *random_draw.timezone;

    // shorthand for getting the current time in the configured timezone
    let now = || Utc::now().with_timezone(&tz);
    // shorthand for getting the next scheduled time in the configured timezone.
    // it is possible to make schedules that have no future times so in that
    // case we just wait 24h
    let next = || {
        random_draw.schedule.upcoming(tz).next().unwrap_or_else(|| {
            warn!("cron schedule produced no upcoming times! falling back to now + 24h");

            now() + Duration::days(1)
        })
    };

    loop {
        let now = now();
        let next = next();
        trace!("next random draw at {:?}", next);

        let sleep_duration = next.signed_duration_since(now);
        debug!("sleeping for {}", sleep_duration);

        // SAFETY: it is not possible for this to be negative because we get now
        // before next
        let duration_std = sleep_duration.to_std().unwrap();
        tokio::time::sleep(duration_std).await;

        if let Err(err) = do_random_draw(random_draw, &data, &http).await {
            error!("failed to run random draw: {:?}", err);
        }
    }
}

pub async fn do_random_draw(
    random_draw: &RandomDrawConfig,
    data: &Data,
    http: impl AsRef<Http>,
) -> Result<()> {
    debug!("running random draw");
    let http = http.as_ref();

    let last_run = data.state.lock().await.last_trigger();
    trace!(?last_run, "last random draw time");
    let index = data.index.lock().await;

    let message = match last_run {
        Some(last_run) => index.random_message_since(last_run),
        None => index.random_message(),
    };
    trace!(?message, "random draw message");

    let Some((channel_id, message_id)) = message else {
        warn!("no messages found for random draw");
        return Ok(());
    };

    let msg = http
        .get_message(channel_id, message_id)
        .await
        .wrap_err("failed to get random draw message")?;

    let prefix = random_draw
        .messages
        .choose(&mut rand::thread_rng())
        .map(|s| s.as_str());
    trace!(?prefix, "random draw prefix");

    random_draw
        .channel_id
        .send_message(
            http,
            CreateMessage::new().content(utils::format_repost_content(msg, prefix)),
        )
        .await
        .wrap_err("failed to send random draw message")?;

    data.state.lock().await.just_triggered().await?;

    Ok(())
}
