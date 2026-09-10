use std::rc::Rc;

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::discord::{purge, token, Control, Events, Filters, LogLine, RunState, Stats};

use super::form::{Field, Form, FormAction, TextSpec, ATTACHMENT_OPTIONS, TEXT_FIELDS};
use super::log::{LogAction, LogModel};

#[function_component(Panel)]
pub fn panel() -> Html {
    let form = use_reducer(Form::default);
    let log = use_reducer(LogModel::default);
    let stats = use_state(Stats::default);
    let state = use_state(|| RunState::Idle);
    let control = use_mut_ref(|| None::<Control>);
    let visible = use_reducer(|| Visible(false));
    let collapsed = use_state(|| false);

    {
        let visible = visible.clone();
        use_effect_with((), move |_| {
            crate::bridge::register(Callback::from(move |_| visible.dispatch(())));
            || ()
        });
    }

    let on_start = start_callback(&form, &log, &stats, &state, &control);
    let on_pause = pause_callback(&state, &control);
    let on_stop = stop_callback(&control);

    let active = state.is_active();
    let panel_class = if *collapsed { "panel collapsed" } else { "panel" };
    let start_label = if form.is_on(Field::DryRun) { "Simular" } else { "Apagar" };

    html! {
            <div class={panel_class} hidden={!visible.0}>
                { header(&visible, &collapsed) }
                <div class="body">
                    { for TEXT_FIELDS.iter().map(|spec| text_field(&form, spec)) }
                    { fill_buttons(&form, &log) }
                    { select_field(&form) }
                    <div class="row">
                        { date_field(&form, Field::After, "Depois de") }
                        { date_field(&form, Field::Before, "Antes de") }
                    </div>
                    <div class="row">
                        { number_field(&form, Field::DeleteDelay, "Delay delete (ms)") }
                        { number_field(&form, Field::SearchDelay, "Delay busca (ms)") }
                    </div>
                    <div class="toggles">
                        { toggle(&form, Field::IncludePinned, "Incluir mensagens fixadas") }
                        { toggle(&form, Field::DryRun, "Simular (só lista, não apaga)") }
                    </div>
                </div>
                <div class="footer">
                    <div class="actions">
                        <button class="action primary" disabled={active} onclick={on_start}>
                            { start_label }
                        </button>
                        <button class="action" disabled={!active} onclick={on_pause}>
                            { if *state == RunState::Paused { "Continuar" } else { "Pausar" } }
                        </button>
                        <button class="action danger" disabled={!active} onclick={on_stop}>
                            { "Parar" }
                        </button>
                    </div>
                    { stats_row(&stats) }
                </div>
                <div class="log">
                    { for log.lines.iter().map(render_line) }
                </div>
            </div>
    }
}

#[derive(PartialEq)]
struct Visible(bool);

impl Reducible for Visible {
    type Action = ();

    fn reduce(self: Rc<Self>, _action: Self::Action) -> Rc<Self> {
        Rc::new(Visible(!self.0))
    }
}

fn header(visible: &UseReducerHandle<Visible>, collapsed: &UseStateHandle<bool>) -> Html {
    let on_collapse = {
        let collapsed = collapsed.clone();
        Callback::from(move |_: MouseEvent| collapsed.set(!*collapsed))
    };
    let on_hide = {
        let visible = visible.clone();
        Callback::from(move |_: MouseEvent| visible.dispatch(()))
    };

    html! {
        <div class="header">
            <span class="title">{ "discord-mass" }</span>
            <span class="badge">{ "wasm" }</span>
            <button class="icon-button" title="Recolher" onclick={on_collapse}>{ "–" }</button>
            <button class="icon-button" title="Esconder" onclick={on_hide}>{ "×" }</button>
        </div>
    }
}

fn text_field(form: &UseReducerHandle<Form>, spec: &'static TextSpec) -> Html {
    let oninput = input_callback(form, spec.field);

    html! {
        <div class="field">
            <label>{ spec.label }</label>
            <input
                type={spec.input_type}
                spellcheck="false"
                value={form.get(spec.field).to_owned()}
                {oninput}
            />
            <span class="hint">{ spec.hint }</span>
        </div>
    }
}

fn number_field(form: &UseReducerHandle<Form>, field: Field, label: &'static str) -> Html {
    let oninput = input_callback(form, field);

    html! {
        <div class="field">
            <label>{ label }</label>
            <input type="number" min="0" step="50" value={form.get(field).to_owned()} {oninput} />
        </div>
    }
}

fn date_field(form: &UseReducerHandle<Form>, field: Field, label: &'static str) -> Html {
    let oninput = input_callback(form, field);

    html! {
        <div class="field">
            <label>{ label }</label>
            <input type="datetime-local" value={form.get(field).to_owned()} {oninput} />
        </div>
    }
}

fn select_field(form: &UseReducerHandle<Form>) -> Html {
    let onchange = {
        let form = form.clone();
        Callback::from(move |event: Event| {
            let value = event.target_unchecked_into::<HtmlSelectElement>().value();
            form.dispatch(FormAction::Set(Field::Has, value));
        })
    };
    let selected = form.get(Field::Has).to_owned();

    html! {
        <div class="field">
            <label>{ "Contém anexo" }</label>
            <select {onchange}>
                { for ATTACHMENT_OPTIONS.iter().map(|(value, label)| html! {
                    <option value={*value} selected={selected == *value}>{ *label }</option>
                }) }
            </select>
        </div>
    }
}

fn toggle(form: &UseReducerHandle<Form>, field: Field, label: &'static str) -> Html {
    let onchange = {
        let form = form.clone();
        Callback::from(move |_: Event| form.dispatch(FormAction::Toggle(field)))
    };

    html! {
        <label class="toggle">
            <input type="checkbox" checked={form.is_on(field)} {onchange} />
            <span>{ label }</span>
        </label>
    }
}

fn fill_buttons(form: &UseReducerHandle<Form>, log: &UseReducerHandle<LogModel>) -> Html {
    let from_location = {
        let form = form.clone();
        let log = log.clone();
        Callback::from(move |_: MouseEvent| match current_channel() {
            Some((guild_id, channel_id)) => {
                form.dispatch(FormAction::Set(Field::GuildId, guild_id));
                form.dispatch(FormAction::Set(Field::ChannelId, channel_id));
            }
            None => log.dispatch(LogAction::Push(LogLine::error(
                "abra um canal antes de usar este atalho".to_owned(),
            ))),
        })
    };

    let own_user = {
        let form = form.clone();
        let log = log.clone();
        Callback::from(move |_: MouseEvent| {
            match token::read().as_deref().and_then(token::user_id) {
                Some(user_id) => form.dispatch(FormAction::Set(Field::AuthorId, user_id)),
                None => log.dispatch(LogAction::Push(LogLine::error(
                    "não consegui ler o seu ID; faça login de novo".to_owned(),
                ))),
            }
        })
    };

    html! {
        <>
            <button class="link-button" onclick={from_location}>{ "usar o canal aberto agora" }</button>
            <button class="link-button" onclick={own_user}>{ "usar a minha conta como autor" }</button>
        </>
    }
}

fn stats_row(stats: &UseStateHandle<Stats>) -> Html {
    let first = if stats.simulated > 0 {
        (stats.simulated, "simuladas")
    } else {
        (stats.deleted, "apagadas")
    };

    let cells = [
        first,
        (stats.skipped, "puladas"),
        (stats.failed, "falhas"),
        (stats.remaining, "restantes"),
    ];

    html! {
        <div class="stats">
            { for cells.iter().map(|(count, label)| html! {
                <span><b>{ count }</b>{ format!(" {label}") }</span>
            }) }
        </div>
    }
}

fn render_line(line: &Rc<LogLine>) -> Html {
    html! {
        <div class={line.kind.css_class()}>{ format!("{}  {}", line.time, line.text) }</div>
    }
}

fn input_callback(form: &UseReducerHandle<Form>, field: Field) -> Callback<InputEvent> {
    let form = form.clone();
    Callback::from(move |event: InputEvent| {
        let value = event.target_unchecked_into::<HtmlInputElement>().value();
        form.dispatch(FormAction::Set(field, value));
    })
}

fn start_callback(
    form: &UseReducerHandle<Form>,
    log: &UseReducerHandle<LogModel>,
    stats: &UseStateHandle<Stats>,
    state: &UseStateHandle<RunState>,
    control: &Rc<std::cell::RefCell<Option<Control>>>,
) -> Callback<MouseEvent> {
    let form = form.clone();
    let log = log.clone();
    let stats = stats.clone();
    let state = state.clone();
    let control = Rc::clone(control);

    Callback::from(move |_: MouseEvent| {
        let filters = match form.to_filters() {
            Ok(filters) => filters,
            Err(problem) => {
                log.dispatch(LogAction::Push(LogLine::error(problem)));
                return;
            }
        };

        if token::read().is_none() {
            log.dispatch(LogAction::Push(LogLine::error(
                "token não encontrado; recarregue a página logado no Discord".to_owned(),
            )));
            return;
        }

        if !filters.dry_run && !confirmed(&filters) {
            return;
        }

        log.dispatch(LogAction::Clear);
        stats.set(Stats::default());

        let handle = Control::new();
        *control.borrow_mut() = Some(handle.clone());
        state.set(RunState::Running);

        let events = Events {
            log: sink(log.clone(), |log: &UseReducerHandle<LogModel>, line: LogLine| {
                log.dispatch(LogAction::Push(line))
            }),
            stats: sink(stats.clone(), |stats: &UseStateHandle<Stats>, value: Stats| {
                stats.set(value)
            }),
            state: sink(state.clone(), |state: &UseStateHandle<RunState>, value: RunState| {
                state.set(value)
            }),
        };

        spawn_local(async move {
            purge::run(filters, handle, events).await;
        });
    })
}

fn sink<H, T, F>(handle: H, apply: F) -> Rc<dyn Fn(T)>
where
    H: 'static,
    T: 'static,
    F: Fn(&H, T) + 'static,
{
    Rc::new(move |value| apply(&handle, value))
}

fn pause_callback(
    state: &UseStateHandle<RunState>,
    control: &Rc<std::cell::RefCell<Option<Control>>>,
) -> Callback<MouseEvent> {
    let state = state.clone();
    let control = Rc::clone(control);

    Callback::from(move |_: MouseEvent| {
        let Some(handle) = control.borrow().clone() else { return };
        let next = match handle.state() {
            RunState::Paused => RunState::Running,
            RunState::Running => RunState::Paused,
            other => other,
        };
        handle.set(next);
        state.set(next);
    })
}

fn stop_callback(control: &Rc<std::cell::RefCell<Option<Control>>>) -> Callback<MouseEvent> {
    let control = Rc::clone(control);

    Callback::from(move |_: MouseEvent| {
        if let Some(handle) = control.borrow().clone() {
            handle.set(RunState::Stopped);
        }
    })
}

fn current_channel() -> Option<(String, String)> {
    let path = web_sys::window()?.location().pathname().ok()?;
    let mut segments = path.split('/').skip(2);
    let scope = segments.next()?.to_owned();
    let channel_id = segments.next()?.to_owned();

    if channel_id.is_empty() {
        return None;
    }

    Some((if scope == "@me" { String::new() } else { scope }, channel_id))
}

fn confirmed(filters: &Filters) -> bool {
    let target = if filters.query.channel_id.is_empty() {
        format!("servidor {}", filters.query.guild_id)
    } else {
        format!("canal {}", filters.query.channel_id)
    };
    let author = if filters.query.author_id.is_empty() {
        "de qualquer autor".to_owned()
    } else {
        format!("do autor {}", filters.query.author_id)
    };
    let message =
        format!("Apagar de verdade as mensagens {author} no {target}?\n\nIsso não tem desfazer.");

    web_sys::window()
        .and_then(|window| window.confirm_with_message(&message).ok())
        .unwrap_or(false)
}
