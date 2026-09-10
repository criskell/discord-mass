mod discord;
mod ui;

use wasm_bindgen::prelude::*;
use web_sys::{ShadowRootInit, ShadowRootMode};

const HOST_ID: &str = "discord-mass-root";

#[wasm_bindgen(start)]
pub fn boot() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("sem window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("sem document"))?;

    if document.get_element_by_id(HOST_ID).is_some() {
        return Ok(());
    }

    let body = document.body().ok_or_else(|| JsValue::from_str("sem body"))?;
    let host = document.create_element("div")?;
    host.set_id(HOST_ID);
    body.append_child(&host)?;

    let shadow = host.attach_shadow(&ShadowRootInit::new(ShadowRootMode::Open))?;

    let style = document.create_element("style")?;
    style.set_text_content(Some(ui::STYLES));
    shadow.append_child(&style)?;

    let mount = document.create_element("div")?;
    shadow.append_child(&mount)?;
    yew::Renderer::<ui::Panel>::with_root(mount).render();

    Ok(())
}
