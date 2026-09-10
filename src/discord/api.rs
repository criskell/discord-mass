use std::fmt;
use std::rc::Rc;

use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestCredentials, RequestInit, Response};

use super::control::{sleep, Control};

const API_PATH: &str = "/api/v9";
const SEARCH_PAGE_SIZE: u8 = 25;
const MAX_ATTEMPTS: u8 = 6;
const SERVER_ERROR_BACKOFF_MS: u32 = 3_000;
const RATE_LIMIT_SAFETY_MS: u32 = 250;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Stopped,
    Http { status: u16, message: String },
    Transport(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(formatter, "token inválido ou expirado"),
            ApiError::Stopped => write!(formatter, "execução interrompida"),
            ApiError::Http { status, message } => write!(formatter, "HTTP {status}: {message}"),
            ApiError::Transport(detail) => write!(formatter, "falha de rede: {detail}"),
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Query {
    pub guild_id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    pub has: String,
    pub min_id: String,
    pub max_id: String,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Author {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "type", default)]
    pub kind: u8,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub hit: bool,
    #[serde(default)]
    pub author: Option<Author>,
    #[serde(default)]
    pub attachments: Vec<serde_json::Value>,
}

pub struct SearchPage {
    pub total_results: u64,
    pub hits: Vec<Message>,
}

pub enum DeleteOutcome {
    Deleted,
    AlreadyGone,
}

#[derive(Deserialize, Default)]
struct SearchBody {
    #[serde(default)]
    total_results: u64,
    #[serde(default)]
    messages: Vec<Vec<Message>>,
}

#[derive(Deserialize, Default)]
struct ErrorBody {
    #[serde(default)]
    retry_after: f64,
    #[serde(default)]
    message: String,
}

pub struct Api {
    token: String,
    on_wait: Rc<dyn Fn(u32, &'static str)>,
}

impl Api {
    pub fn new(token: String, on_wait: Rc<dyn Fn(u32, &'static str)>) -> Self {
        Self { token, on_wait }
    }

    pub async fn search(&self, query: &Query, control: &Control) -> Result<SearchPage, ApiError> {
        let scope = if query.guild_id.is_empty() {
            format!("/channels/{}/messages/search", query.channel_id)
        } else {
            format!("/guilds/{}/messages/search", query.guild_id)
        };

        let path = format!("{scope}?{}", build_query(query));
        let (status, body) = self.send("GET", &path, control).await?;

        if status == 204 || body.is_empty() {
            return Ok(SearchPage { total_results: 0, hits: Vec::new() });
        }

        let parsed: SearchBody = serde_json::from_str(&body)
            .map_err(|error| ApiError::Transport(error.to_string()))?;

        Ok(SearchPage {
            total_results: parsed.total_results,
            hits: pick_hits(parsed.messages),
        })
    }

    pub async fn delete(
        &self,
        channel_id: &str,
        message_id: &str,
        control: &Control,
    ) -> Result<DeleteOutcome, ApiError> {
        let path = format!("/channels/{channel_id}/messages/{message_id}");
        let (status, _) = self.send("DELETE", &path, control).await?;
        Ok(if status == 404 {
            DeleteOutcome::AlreadyGone
        } else {
            DeleteOutcome::Deleted
        })
    }

    async fn send(
        &self,
        method: &str,
        path: &str,
        control: &Control,
    ) -> Result<(u16, String), ApiError> {
        for attempt in 1..=MAX_ATTEMPTS {
            if control.is_stopped() {
                return Err(ApiError::Stopped);
            }

            let response = self.fetch(method, path).await?;
            let status = response.status();

            if status == 401 {
                return Err(ApiError::Unauthorized);
            }

            if status == 429 || status == 202 {
                let body = read_text(&response).await?;
                let reason = if status == 429 { "rate limit" } else { "índice da busca" };
                if !self.wait(retry_after_ms(&body), reason, control).await {
                    return Err(ApiError::Stopped);
                }
                continue;
            }

            if status >= 500 {
                let backoff = SERVER_ERROR_BACKOFF_MS * u32::from(attempt);
                if !self.wait(backoff, "erro do servidor", control).await {
                    return Err(ApiError::Stopped);
                }
                continue;
            }

            let body = read_text(&response).await?;

            if status >= 400 && status != 404 {
                return Err(ApiError::Http { status, message: error_message(&body) });
            }

            self.respect_bucket(&response, control).await;
            return Ok((status, body));
        }

        Err(ApiError::Transport("número máximo de tentativas atingido".to_owned()))
    }

    async fn fetch(&self, method: &str, path: &str) -> Result<Response, ApiError> {
        let window = web_sys::window().ok_or_else(|| ApiError::Transport("sem window".into()))?;
        let origin = window.location().origin().map_err(to_transport_error)?;

        let headers = Headers::new().map_err(to_transport_error)?;
        headers.set("authorization", &self.token).map_err(to_transport_error)?;

        let init = RequestInit::new();
        init.set_method(method);
        init.set_headers(&headers);
        init.set_credentials(RequestCredentials::Omit);

        let request = Request::new_with_str_and_init(&format!("{origin}{API_PATH}{path}"), &init)
            .map_err(to_transport_error)?;

        let value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(to_transport_error)?;

        value.dyn_into::<Response>().map_err(|_| {
            ApiError::Transport("resposta inesperada do fetch".to_owned())
        })
    }

    async fn respect_bucket(&self, response: &Response, control: &Control) {
        let headers = response.headers();
        if headers.get("x-ratelimit-remaining").ok().flatten().as_deref() != Some("0") {
            return;
        }

        let reset_after = headers
            .get("x-ratelimit-reset-after")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);

        sleep((reset_after * 1000.0) as u32 + RATE_LIMIT_SAFETY_MS, control).await;
    }

    async fn wait(&self, milliseconds: u32, reason: &'static str, control: &Control) -> bool {
        (self.on_wait)(milliseconds, reason);
        sleep(milliseconds, control).await
    }
}

fn pick_hits(groups: Vec<Vec<Message>>) -> Vec<Message> {
    let mut hits: Vec<Message> = groups
        .into_iter()
        .filter_map(|group| {
            group
                .iter()
                .find(|message| message.hit)
                .cloned()
                .or_else(|| group.into_iter().next())
        })
        .collect();

    hits.sort_by_key(|message| message.id.parse::<u64>().unwrap_or(0));
    hits
}

fn build_query(query: &Query) -> String {
    let channel_scope = if query.guild_id.is_empty() { "" } else { query.channel_id.as_str() };

    let mut parts = vec![
        "include_nsfw=true".to_owned(),
        "sort_by=timestamp".to_owned(),
        "sort_order=asc".to_owned(),
        "offset=0".to_owned(),
        format!("limit={SEARCH_PAGE_SIZE}"),
    ];

    let optional = [
        ("channel_id", channel_scope),
        ("author_id", query.author_id.as_str()),
        ("content", query.content.as_str()),
        ("has", query.has.as_str()),
        ("min_id", query.min_id.as_str()),
        ("max_id", query.max_id.as_str()),
    ];

    for (key, value) in optional {
        if !value.is_empty() {
            let encoded: String = js_sys::encode_uri_component(value).into();
            parts.push(format!("{key}={encoded}"));
        }
    }

    parts.join("&")
}

async fn read_text(response: &Response) -> Result<String, ApiError> {
    let promise = response.text().map_err(to_transport_error)?;
    let value = JsFuture::from(promise).await.map_err(to_transport_error)?;
    Ok(value.as_string().unwrap_or_default())
}

fn retry_after_ms(body: &str) -> u32 {
    let parsed: ErrorBody = serde_json::from_str(body).unwrap_or_default();
    let seconds = if parsed.retry_after > 0.0 { parsed.retry_after } else { 1.0 };
    (seconds * 1000.0) as u32 + RATE_LIMIT_SAFETY_MS
}

fn error_message(body: &str) -> String {
    let parsed: ErrorBody = serde_json::from_str(body).unwrap_or_default();
    if parsed.message.is_empty() {
        "sem detalhe".to_owned()
    } else {
        parsed.message
    }
}

fn to_transport_error(value: JsValue) -> ApiError {
    ApiError::Transport(
        value
            .as_string()
            .or_else(|| js_sys::Reflect::get(&value, &JsValue::from_str("message")).ok()?.as_string())
            .unwrap_or_else(|| "erro desconhecido".to_owned()),
    )
}
