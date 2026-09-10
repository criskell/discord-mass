use std::rc::Rc;

use super::api::{Api, ApiError, DeleteOutcome, Query};
use super::control::{sleep, wait_while_paused, Control, RunState};
use super::filter::{classify, describe, Verdict};

pub type Sink<T> = Rc<dyn Fn(T)>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Delete,
    Skip,
    Dry,
    Error,
}

impl LogKind {
    pub fn css_class(self) -> &'static str {
        match self {
            LogKind::Delete => "delete",
            LogKind::Skip => "skip",
            LogKind::Dry => "dry",
            LogKind::Error => "error",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct LogLine {
    pub kind: LogKind,
    pub time: String,
    pub text: String,
}

impl LogLine {
    pub fn new(kind: LogKind, text: String) -> Self {
        Self { kind, time: now(), text }
    }

    pub fn error(text: String) -> Self {
        Self::new(LogKind::Error, text)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub deleted: u64,
    pub simulated: u64,
    pub skipped: u64,
    pub failed: u64,
    pub remaining: u64,
}

#[derive(Clone, Default, PartialEq)]
pub struct Filters {
    pub query: Query,
    pub include_pinned: bool,
    pub dry_run: bool,
    pub delete_delay_ms: u32,
    pub search_delay_ms: u32,
}

pub struct Events {
    pub log: Sink<LogLine>,
    pub stats: Sink<Stats>,
    pub state: Sink<RunState>,
}

pub async fn run(filters: Filters, control: Control, events: Events) -> Stats {
    let mut stats = Stats::default();
    let api = build_api(&events);

    let mode = if filters.dry_run {
        "simulação: nada será apagado"
    } else {
        "execução: as mensagens serão apagadas"
    };
    (events.log)(LogLine::new(LogKind::Dry, mode.to_owned()));

    let outcome = drive(&api, &filters, &control, &events, &mut stats).await;

    let final_state = match outcome {
        Ok(()) if control.is_stopped() => RunState::Stopped,
        Ok(()) => RunState::Finished,
        Err(ApiError::Stopped) => RunState::Stopped,
        Err(error) => {
            (events.log)(LogLine::error(error.to_string()));
            RunState::Failed
        }
    };

    control.set(final_state);
    (events.state)(final_state);
    (events.log)(LogLine::new(LogKind::Dry, summary(&stats, filters.dry_run)));
    stats
}

fn summary(stats: &Stats, dry_run: bool) -> String {
    let head = if dry_run {
        format!("fim da simulação: {} seriam apagadas", stats.simulated)
    } else {
        format!("fim: {} apagadas", stats.deleted)
    };
    format!("{head}, {} puladas, {} falhas", stats.skipped, stats.failed)
}

async fn drive(
    api: &Api,
    filters: &Filters,
    control: &Control,
    events: &Events,
    stats: &mut Stats,
) -> Result<(), ApiError> {
    let mut query = filters.query.clone();

    loop {
        if control.is_stopped() {
            return Ok(());
        }

        let page = api.search(&query, control).await?;
        stats.remaining = page.total_results;
        (events.stats)(*stats);

        if page.hits.is_empty() {
            return Ok(());
        }

        for message in page.hits {
            if !wait_while_paused(control).await {
                return Ok(());
            }
            query.min_id = message.id.clone();
            process(api, filters, control, events, stats, &message).await?;
        }

        if !sleep(filters.search_delay_ms, control).await {
            return Ok(());
        }
    }
}

async fn process(
    api: &Api,
    filters: &Filters,
    control: &Control,
    events: &Events,
    stats: &mut Stats,
    message: &super::api::Message,
) -> Result<(), ApiError> {
    if let Verdict::Skip(reason) = classify(message, &filters.query.author_id, filters.include_pinned)
    {
        stats.skipped += 1;
        let text = format!("pulada ({reason}) — {}", describe(message));
        (events.log)(LogLine::new(LogKind::Skip, text));
        (events.stats)(*stats);
        return Ok(());
    }

    if filters.dry_run {
        stats.simulated += 1;
        let text = format!("apagaria — {}", describe(message));
        (events.log)(LogLine::new(LogKind::Dry, text));
        (events.stats)(*stats);
        return Ok(());
    }

    match api.delete(&message.channel_id, &message.id, control).await {
        Ok(outcome) => {
            stats.deleted += 1;
            let prefix = match outcome {
                DeleteOutcome::Deleted => "apagada",
                DeleteOutcome::AlreadyGone => "já não existia",
            };
            (events.log)(LogLine::new(LogKind::Delete, format!("{prefix} — {}", describe(message))));
        }
        Err(ApiError::Stopped) => return Err(ApiError::Stopped),
        Err(ApiError::Unauthorized) => return Err(ApiError::Unauthorized),
        Err(error) => {
            stats.failed += 1;
            let text = format!("falhou — {} ({error})", describe(message));
            (events.log)(LogLine::error(text));
        }
    }

    (events.stats)(*stats);
    sleep(filters.delete_delay_ms, control).await;
    Ok(())
}

fn build_api(events: &Events) -> Api {
    let token = super::token::read().unwrap_or_default();
    let log = Rc::clone(&events.log);

    Api::new(
        token,
        Rc::new(move |milliseconds, reason| {
            let seconds = f64::from(milliseconds) / 1000.0;
            let text = format!("aguardando {seconds:.1}s ({reason})");
            log(LogLine::new(LogKind::Skip, text));
        }),
    )
}

fn now() -> String {
    js_sys::Date::new_0()
        .to_locale_time_string("pt-BR")
        .as_string()
        .unwrap_or_default()
}
