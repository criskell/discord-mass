pub fn read() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let stored = storage.get_item("token").ok()??;
    Some(stored.trim_matches('"').to_owned())
}

pub fn user_id(token: &str) -> Option<String> {
    let encoded = token.split('.').next()?;
    let padded = to_standard_base64(encoded);
    web_sys::window()?.atob(&padded).ok()
}

fn to_standard_base64(value: &str) -> String {
    let mut standard = value.replace('-', "+").replace('_', "/");
    while standard.len() % 4 != 0 {
        standard.push('=');
    }
    standard
}
