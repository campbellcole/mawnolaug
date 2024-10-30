use poise::serenity_prelude::Message;

pub fn format_repost_content(message: Message, prefix: Option<impl ToString>) -> String {
    let mut content = prefix.map(|p| p.to_string()).unwrap_or_default();

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
