use chrono::{DateTime, Utc};
use color_eyre::eyre::{bail, Result, WrapErr};
use lazy_regex::regex_replace_all;
use poise::serenity_prelude::{Channel, ChannelId, Context, EditChannel, Mentionable, Message};
use tokio::sync::MutexGuard;

use crate::{state::State, DataHolderKey};

/// Generates a Discord timestamp string from the provided timestamp and format.
///
/// Discord timestamp strings are of the format `<t:TIMESTAMP:FORMAT>`, where `TIMESTAMP` is the
/// timestamp in seconds since the Unix epoch and `FORMAT` is one of:
/// - `t`: Short time format (e.g. 16:20)
/// - `T`: Long time format (e.g. 16:20:30)
/// - `d`: Short date format (e.g. 20/04/2021)
/// - `D`: Long date format (e.g. 20 April 2021)
/// - `f`: Short date and time format (e.g. 20 April 2021 16:20)
/// - `F`: Long date and time format (e.g. Tuesday, 20 April 2021 16:20)
/// - `R`: Relative time format (e.g. 2 months ago)
fn generate_discord_timestamp(timestamp: DateTime<Utc>, format: &str) -> String {
    format!("<t:{}:{}>", timestamp.timestamp(), format)
}

/// Apply our custom formatting to the prefix. The following replacements are made:
/// - `{author}`: A mention of the message author
/// - `{author.name}`: The name of the message author
/// - `{author.id}`: The ID of the message author
/// - `{channel}`: A mention of the author's monologue channel
/// - `{channel.id}`: The ID of the author's monologue channel
/// - `{timestamp:<format>}`: The timestamp of the message with the specified format
///
/// There is currently no `{channel.name}` replacement because that requires an additional API call
fn format_prefix(mut prefix: String, message: &Message) -> String {
    let channel_id = &message.channel_id;
    let author = &message.author;

    prefix = prefix
        .replace("{author}", &author.mention().to_string())
        .replace("{author.name}", &author.name)
        .replace("{author.id}", &author.id.to_string())
        .replace("{channel}", &channel_id.mention().to_string())
        .replace("{channel.id}", &channel_id.to_string());

    prefix = regex_replace_all!(
        r#"\{timestamp:(?P<format>[tTdDfFR])\}"#,
        &prefix,
        |_, format| { generate_discord_timestamp(*message.timestamp, format) }
    )
    .to_string();

    prefix
}

pub fn format_repost_content(message: Message, prefix: Option<&str>) -> String {
    let prefix = prefix
        .map(|p| format_prefix(p.to_string(), &message))
        .unwrap_or_default();

    let mut content = prefix;

    if !message.content.is_empty() {
        content.push_str(&format!(
            "{}{}",
            if !content.is_empty() { "\n\n" } else { "" },
            message.content
        ));
    }

    for attachment in message.attachments {
        content.push_str(&format!(
            "{}{}",
            if !content.is_empty() { "\n" } else { "" },
            attachment.url
        ));
    }

    content
}

/// Move the provided channel to the top of its parent.
///
/// This function doesn't need to know the category ID because Discord automatically manages the
/// sort order within categories, so the positions are only relative to other channels in the
/// category. We don't call this function unless the user has set a category in the config because
/// it almost certainly won't work right unless every channel is contained within a category.
pub async fn move_channel_to_top(cx: Context, channel_id: ChannelId) -> Result<()> {
    let Channel::Guild(mut channel) = channel_id
        .to_channel(&cx.http)
        .await
        .wrap_err("failed to get channel")?
    else {
        bail!("provided channel is not a guild channel");
    };
    debug!("moving channel to top: {}", channel.name);

    let data = cx.data.read().await;
    let holder = data.get::<DataHolderKey>().unwrap();
    let mut state = holder.state.lock().await;

    let mut next_pos = state.next_position();
    trace!("next position: {}", next_pos);

    if next_pos == channel.position - 1 {
        trace!("channel is already at the top");
        return Ok(());
    }

    if next_pos == 0 {
        trace!("next position is zero, resetting channel positions");
        initialize_channel_positions(&cx, &mut state).await?;

        next_pos = state.next_position();
        trace!("new next position: {}", next_pos);
    }

    channel
        .edit(&cx.http, EditChannel::new().position(next_pos))
        .await?;

    state.set_channel_position(channel.id, next_pos).await?;

    Ok(())
}

/// Initialize the positions of all channels in the provided category.
///
/// This moves all channels to `u16::MAX - N` where N is the current channel position relative to
/// the others. This provides a clean slate for ordering the channels later on. Each time a channel
/// is updated, we move it to the top of the category by taking the current highest position and
/// subtracting 1.
///
/// This function maintains the current order of the channels. For example, if the channels are
/// ordered like:
/// - Channel 1: 1
/// - Channel 2: 2
/// - Channel 3: 3
///
/// After this function executes, they will have the order:
/// - Channel 1: u16::MAX - 2
/// - Channel 2: u16::MAX - 1
/// - Channel 3: u16::MAX
async fn initialize_channel_positions(
    cx: &Context,
    state: &mut MutexGuard<'_, State>,
) -> Result<()> {
    let channel_ids = state.get_channels();

    let mut channels = Vec::with_capacity(channel_ids.len());

    for channel_id in channel_ids {
        let Channel::Guild(channel) = channel_id
            .to_channel(&cx.http)
            .await
            .wrap_err("failed to get channel")?
        else {
            continue;
        };

        channels.push(channel);
    }

    channels.sort_by_key(|c| c.position);
    channels.reverse();

    // we could hypothetically use the
    // https://discord.com/developers/docs/resources/guild#modify-guild-channel-positions endpoint
    // to batch edit the channel positions, but apparently it can mess with other channels'
    // positions if they are unspecified and fetching a full guild channel object for every channel
    // is almost certainly more expensive than this
    for (position, mut channel) in channels.into_iter().enumerate() {
        let new_pos = u16::MAX - position as u16;
        trace!("setting channel {} to position {}", channel.name, new_pos);

        channel
            .edit(&cx.http, EditChannel::new().position(new_pos))
            .await?;

        state.set_channel_position(channel.id, new_pos).await?;
    }

    Ok(())
}
