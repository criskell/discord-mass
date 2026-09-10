use std::cell::Cell;
use std::rc::Rc;

use gloo_timers::future::TimeoutFuture;

const TICK_MS: u32 = 80;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunState {
    Idle,
    Running,
    Paused,
    Stopped,
    Finished,
    Failed,
}

impl RunState {
    pub fn is_active(self) -> bool {
        matches!(self, RunState::Running | RunState::Paused)
    }
}

#[derive(Clone)]
pub struct Control(Rc<Cell<RunState>>);

impl Control {
    pub fn new() -> Self {
        Self(Rc::new(Cell::new(RunState::Running)))
    }

    pub fn state(&self) -> RunState {
        self.0.get()
    }

    pub fn set(&self, state: RunState) {
        self.0.set(state);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.get() == RunState::Stopped
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn sleep(milliseconds: u32, control: &Control) -> bool {
    let mut remaining = milliseconds;
    while remaining > 0 {
        if control.is_stopped() {
            return false;
        }
        let slice = remaining.min(TICK_MS);
        TimeoutFuture::new(slice).await;
        remaining -= slice;
    }
    !control.is_stopped()
}

pub async fn wait_while_paused(control: &Control) -> bool {
    while control.state() == RunState::Paused {
        TimeoutFuture::new(TICK_MS).await;
    }
    !control.is_stopped()
}
