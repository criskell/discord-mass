use std::cell::{Cell, RefCell};

use wasm_bindgen::prelude::*;
use yew::Callback;

thread_local! {
    static TOGGLE: RefCell<Option<Callback<()>>> = RefCell::new(None);
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

pub fn register(toggle: Callback<()>) {
    if PENDING.with(Cell::take) {
        toggle.emit(());
    }
    TOGGLE.with(|slot| *slot.borrow_mut() = Some(toggle));
}

#[wasm_bindgen]
pub fn toggle_panel() {
    let registered = TOGGLE.with(|slot| slot.borrow().clone());
    match registered {
        Some(toggle) => toggle.emit(()),
        None => PENDING.with(|pending| pending.set(!pending.get())),
    }
}
