use lazy_regex::regex_replace_all;
use poise::serenity_prelude::{Mentionable, Message};

use crate::config::AppConfig;

/// Apply our custom formatting to the prefix. The following replacements are made:
/// - `{author}`: A mention of the message author
/// - `{author.name}`: The name of the message author
/// - `{author.id}`: The ID of the message author
/// - `{channel}`: A mention of the author's monologue channel
/// - `{channel.id}`: The ID of the author's monologue channel
/// - `{timestamp:<format>}`: The timestamp of the message with the specified format
///
/// There is currently no `{channel.name}` replacement because that requires an additional API call
fn format_prefix(config: &AppConfig, mut prefix: String, message: &Message) -> String {
    let channel_id = &message.channel_id;
    let author = &message.author;

    prefix = prefix
        .replace("{author}", &author.mention().to_string())
        .replace("{author.name}", &author.name)
        .replace("{author.id}", &author.id.to_string())
        .replace("{channel}", &channel_id.mention().to_string())
        .replace("{channel.id}", &channel_id.to_string());

    prefix = regex_replace_all!(
        r#"\{timestamp:(?P<format>[^}]+)\}"#,
        &prefix,
        |_, format| {
            let timezone = config.timezone;
            message
                .timestamp
                .with_timezone(&*timezone)
                .format(format)
                .to_string()
        }
    )
    .to_string();

    prefix
}

pub fn format_repost_content(
    config: &AppConfig,
    message: Message,
    prefix: Option<impl ToString>,
) -> String {
    let prefix = prefix
        .map(|p| format_prefix(config, p.to_string(), &message))
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
