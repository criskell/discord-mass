use std::rc::Rc;

use super::api::{Api, ApiError, DeleteOutcome, Message, Query};
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
    let mut session = Session::new(filters, control, events);
    session.announce_mode();
    let outcome = session.drive().await;
    session.finish(outcome)
}

struct Session {
    api: Api,
    filters: Filters,
    control: Control,
    events: Events,
    stats: Stats,
}

impl Session {
    fn new(filters: Filters, control: Control, events: Events) -> Self {
        let api = build_api(&events);
        Self { api, filters, control, events, stats: Stats::default() }
    }

    fn announce_mode(&self) {
        let mode = if self.filters.dry_run {
            "simulação: nada será apagado"
        } else {
            "execução: as mensagens serão apagadas"
        };
        self.log(LogKind::Dry, mode.to_owned());
    }

    async fn drive(&mut self) -> Result<(), ApiError> {
        let mut query = self.filters.query.clone();

        loop {
            if self.control.is_stopped() {
                return Ok(());
            }

            let page = self.api.search(&query, &self.control).await?;
            self.stats.remaining = page.total_results;
            self.emit_stats();

            if page.hits.is_empty() {
                return Ok(());
            }

            for message in page.hits {
                if !wait_while_paused(&self.control).await {
                    return Ok(());
                }
                query.min_id = message.id.clone();
                self.process(&message).await?;
            }

            if !sleep(self.filters.search_delay_ms, &self.control).await {
                return Ok(());
            }
        }
    }

    async fn process(&mut self, message: &Message) -> Result<(), ApiError> {
        let verdict = classify(message, &self.filters.query.author_id, self.filters.include_pinned);

        if let Verdict::Skip(reason) = verdict {
            self.stats.skipped += 1;
            self.log(LogKind::Skip, format!("pulada ({reason}) — {}", describe(message)));
            self.emit_stats();
            return Ok(());
        }

        if self.filters.dry_run {
            self.stats.simulated += 1;
            self.log(LogKind::Dry, format!("apagaria — {}", describe(message)));
            self.emit_stats();
            return Ok(());
        }

        match self.api.delete(&message.channel_id, &message.id, &self.control).await {
            Ok(outcome) => {
                self.stats.deleted += 1;
                let prefix = match outcome {
                    DeleteOutcome::Deleted => "apagada",
                    DeleteOutcome::AlreadyGone => "já não existia",
                };
                self.log(LogKind::Delete, format!("{prefix} — {}", describe(message)));
            }
            Err(ApiError::Stopped) => return Err(ApiError::Stopped),
            Err(ApiError::Unauthorized) => return Err(ApiError::Unauthorized),
            Err(error) => {
                self.stats.failed += 1;
                self.log(LogKind::Error, format!("falhou — {} ({error})", describe(message)));
            }
        }

        self.emit_stats();
        sleep(self.filters.delete_delay_ms, &self.control).await;
        Ok(())
    }

    fn finish(&mut self, outcome: Result<(), ApiError>) -> Stats {
        let final_state = match outcome {
            Ok(()) if self.control.is_stopped() => RunState::Stopped,
            Ok(()) => RunState::Finished,
            Err(ApiError::Stopped) => RunState::Stopped,
            Err(error) => {
                self.log(LogKind::Error, error.to_string());
                RunState::Failed
            }
        };

        self.control.set(final_state);
        (self.events.state)(final_state);
        self.log(LogKind::Dry, self.summary());
        self.stats
    }

    fn summary(&self) -> String {
        let head = if self.filters.dry_run {
            format!("fim da simulação: {} seriam apagadas", self.stats.simulated)
        } else {
            format!("fim: {} apagadas", self.stats.deleted)
        };
        format!("{head}, {} puladas, {} falhas", self.stats.skipped, self.stats.failed)
    }

    fn emit_stats(&self) {
        (self.events.stats)(self.stats);
    }

    fn log(&self, kind: LogKind, text: String) {
        (self.events.log)(LogLine::new(kind, text));
    }
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
