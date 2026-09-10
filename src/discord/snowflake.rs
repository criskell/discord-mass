const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;
const TIMESTAMP_SHIFT: u32 = 22;

pub fn from_local_datetime(value: &str) -> Option<String> {
    let millis = parse_local_datetime(value)?;
    let offset = millis - DISCORD_EPOCH_MS;
    if offset <= 0 {
        return Some("0".to_owned());
    }
    Some(((offset as u64) << TIMESTAMP_SHIFT).to_string())
}

fn parse_local_datetime(value: &str) -> Option<i64> {
    let millis = js_sys::Date::parse(value);
    if millis.is_nan() {
        None
    } else {
        Some(millis as i64)
    }
}

pub fn is_after(candidate: &str, reference: &str) -> bool {
    match (candidate.parse::<u64>(), reference.parse::<u64>()) {
        (Ok(left), Ok(right)) => left > right,
        _ => false,
    }
}
