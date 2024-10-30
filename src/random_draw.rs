use std::{str::FromStr, sync::Arc};

use chrono::{Duration, Utc};
use color_eyre::eyre::{Context, Result};
use poise::serenity_prelude::{CreateMessage, Http};
use rand::seq::SliceRandom;

use crate::{config::RandomDrawConfig, utils, Data};

pub async fn random_draw_task(data: Arc<Data>, http: Arc<Http>) {
    let Some(random_draw) = &data.config.random_draw else {
        debug!("random draw is disabled");
        return;
    };

    debug!("starting random draw task");

    let tz = match random_draw.timezone {
        Some(ref tz) => *tz,
        None => {
            let iana = match iana_time_zone::get_timezone() {
                Ok(i) => i,
                Err(err) => {
                    error!("please set the timezone in the config file. failed to read system timezone: {}", err);
                    return;
                }
            };

            match chrono_tz::Tz::from_str(&iana) {
                Ok(tz) => tz,
                Err(err) => {
                    error!(
                        "please set the timezone in the config file. failed to parse timezone: {}",
                        err
                    );
                    return;
                }
            }
        }
    };

    let now = || Utc::now().with_timezone(&tz);
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

        // SAFETY: it is not possible for this to be negative because we get now before next
        let duration_std = sleep_duration.to_std().unwrap();
        tokio::time::sleep(duration_std).await;

        if let Err(err) = do_random_draw(random_draw, &data, &http).await {
            error!("failed to run random draw: {}", err);
        }
    }
}

pub async fn do_random_draw(
    random_draw: &RandomDrawConfig,
    data: &Arc<Data>,
    http: impl AsRef<Http>,
) -> Result<()> {
    debug!("running random draw");
    let http = http.as_ref();

    let last_run = data.state.lock().await.last_trigger();
    let index = data.index.lock().await;
    let Some((channel_id, message_id)) = last_run.and_then(|last| index.random_message_since(last))
    else {
        warn!("no messages found for random draw");
        return Ok(());
    };

    let msg = http
        .get_message(channel_id, message_id)
        .await
        .wrap_err("failed to get random draw message")?;

    let prefix = random_draw
        .messages
        .as_ref()
        .and_then(|messages| messages.choose(&mut rand::thread_rng()).map(|s| s.as_str()));

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
