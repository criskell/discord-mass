use std::rc::Rc;

use yew::Reducible;

use crate::discord::{snowflake, token, Filters, Query};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    GuildId,
    ChannelId,
    AuthorId,
    Content,
    Has,
    After,
    Before,
    DeleteDelay,
    SearchDelay,
    IncludePinned,
    DryRun,
}

pub struct TextSpec {
    pub field: Field,
    pub label: &'static str,
    pub hint: &'static str,
    pub input_type: &'static str,
}

pub const TEXT_FIELDS: [TextSpec; 4] = [
    TextSpec { field: Field::GuildId, label: "Servidor (guild ID)", hint: "vazio = mensagem direta", input_type: "text" },
    TextSpec { field: Field::ChannelId, label: "Canal (channel ID)", hint: "vazio = servidor inteiro", input_type: "text" },
    TextSpec { field: Field::AuthorId, label: "Autor (user ID)", hint: "quem escreveu as mensagens", input_type: "text" },
    TextSpec { field: Field::Content, label: "Contém o texto", hint: "opcional", input_type: "text" },
];

pub const ATTACHMENT_OPTIONS: [(&str, &str); 8] = [
    ("", "qualquer coisa"),
    ("link", "link"),
    ("embed", "embed"),
    ("file", "arquivo"),
    ("image", "imagem"),
    ("video", "vídeo"),
    ("sound", "áudio"),
    ("sticker", "sticker"),
];

#[derive(Clone, PartialEq)]
pub struct Form {
    pub guild_id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    pub has: String,
    pub after: String,
    pub before: String,
    pub delete_delay: String,
    pub search_delay: String,
    pub include_pinned: bool,
    pub dry_run: bool,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            guild_id: String::new(),
            channel_id: String::new(),
            author_id: String::new(),
            content: String::new(),
            has: String::new(),
            after: String::new(),
            before: String::new(),
            delete_delay: "900".to_owned(),
            search_delay: "1200".to_owned(),
            include_pinned: false,
            dry_run: true,
        }
    }
}

pub enum FormAction {
    Set(Field, String),
    Toggle(Field),
    Autofill,
}

impl Reducible for Form {
    type Action = FormAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut next = (*self).clone();
        match action {
            FormAction::Set(field, value) => next.set(field, value),
            FormAction::Toggle(field) => next.toggle(field),
            FormAction::Autofill => next.autofill(),
        }
        Rc::new(next)
    }
}

impl Form {
    pub fn get(&self, field: Field) -> &str {
        match field {
            Field::GuildId => &self.guild_id,
            Field::ChannelId => &self.channel_id,
            Field::AuthorId => &self.author_id,
            Field::Content => &self.content,
            Field::Has => &self.has,
            Field::After => &self.after,
            Field::Before => &self.before,
            Field::DeleteDelay => &self.delete_delay,
            Field::SearchDelay => &self.search_delay,
            Field::IncludePinned | Field::DryRun => "",
        }
    }

    pub fn is_on(&self, field: Field) -> bool {
        match field {
            Field::IncludePinned => self.include_pinned,
            Field::DryRun => self.dry_run,
            _ => false,
        }
    }

    fn set(&mut self, field: Field, value: String) {
        match field {
            Field::GuildId => self.guild_id = value,
            Field::ChannelId => self.channel_id = value,
            Field::AuthorId => self.author_id = value,
            Field::Content => self.content = value,
            Field::Has => self.has = value,
            Field::After => self.after = value,
            Field::Before => self.before = value,
            Field::DeleteDelay => self.delete_delay = value,
            Field::SearchDelay => self.search_delay = value,
            Field::IncludePinned | Field::DryRun => {}
        }
    }

    fn autofill(&mut self) {
        if let Some((guild_id, channel_id)) = current_channel() {
            self.guild_id = guild_id;
            self.channel_id = channel_id;
        }
        if self.author_id.is_empty() {
            if let Some(user_id) = own_user_id() {
                self.author_id = user_id;
            }
        }
    }

    fn toggle(&mut self, field: Field) {
        match field {
            Field::IncludePinned => self.include_pinned = !self.include_pinned,
            Field::DryRun => self.dry_run = !self.dry_run,
            _ => {}
        }
    }

    pub fn to_filters(&self) -> Result<Filters, String> {
        let min_id = to_snowflake(&self.after)?;
        let max_id = to_snowflake(&self.before)?;

        if self.guild_id.trim().is_empty() && self.channel_id.trim().is_empty() {
            return Err("Informe um canal (para DM) ou um servidor.".to_owned());
        }
        if !min_id.is_empty() && !max_id.is_empty() && !snowflake::is_after(&max_id, &min_id) {
            return Err("A data final precisa ser posterior à inicial.".to_owned());
        }

        Ok(Filters {
            query: Query {
                guild_id: self.guild_id.trim().to_owned(),
                channel_id: self.channel_id.trim().to_owned(),
                author_id: self.author_id.trim().to_owned(),
                content: self.content.trim().to_owned(),
                has: self.has.clone(),
                min_id,
                max_id,
            },
            include_pinned: self.include_pinned,
            dry_run: self.dry_run,
            delete_delay_ms: parse_delay(&self.delete_delay),
            search_delay_ms: parse_delay(&self.search_delay),
        })
    }
}

pub fn current_channel() -> Option<(String, String)> {
    let path = web_sys::window()?.location().pathname().ok()?;
    let mut segments = path.split('/').skip(2);
    let scope = segments.next()?.to_owned();
    let channel_id = segments.next()?.to_owned();

    if channel_id.is_empty() {
        return None;
    }

    Some((if scope == "@me" { String::new() } else { scope }, channel_id))
}

pub fn own_user_id() -> Option<String> {
    token::read().as_deref().and_then(token::user_id)
}

fn to_snowflake(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    snowflake::from_local_datetime(value).ok_or_else(|| format!("data inválida: {value}"))
}

fn parse_delay(value: &str) -> u32 {
    value.trim().parse().unwrap_or(0)
}
