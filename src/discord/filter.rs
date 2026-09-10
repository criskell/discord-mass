use super::api::Message;

const DELETABLE_TYPES: [u8; 5] = [0, 6, 19, 20, 23];

pub enum Verdict {
    Deletable,
    Skip(&'static str),
}

pub fn classify(message: &Message, author_id: &str, include_pinned: bool) -> Verdict {
    if message.pinned && !include_pinned {
        return Verdict::Skip("fixada");
    }
    if !DELETABLE_TYPES.contains(&message.kind) {
        return Verdict::Skip("mensagem de sistema");
    }
    if !author_id.is_empty() && message.author.as_ref().map(|a| a.id.as_str()) != Some(author_id) {
        return Verdict::Skip("outro autor");
    }
    Verdict::Deletable
}

pub fn describe(message: &Message) -> String {
    let author = message
        .author
        .as_ref()
        .map(|a| a.username.as_str())
        .unwrap_or("?");

    let text: String = message.content.replace('\n', " ").chars().take(60).collect();
    let body = if text.is_empty() { "(sem texto)".to_owned() } else { text };

    match message.attachments.len() {
        0 => format!("{author}: {body}"),
        count => format!("{author}: {body} [{count} anexo(s)]"),
    }
}
