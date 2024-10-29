use color_eyre::eyre::Result;
use poise::serenity_prelude::Message;

pub async fn repost_message(message: Message, prefix: impl ToString) -> Result<String> {
    let mut content = prefix.to_string();

    if !message.content.is_empty() {
        content.push_str(&format!("\n\n{}", message.content));
    }

    for attachment in message.attachments {
        content.push_str(&format!("\n{}", attachment.url));
    }

    Ok(content)
}
