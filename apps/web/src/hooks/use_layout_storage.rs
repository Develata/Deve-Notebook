pub(crate) fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min, max)
}

pub(crate) fn read_width(key: &str) -> Option<i32> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let val = storage.get_item(key).ok()??;
    val.parse::<i32>().ok()
}

pub(crate) fn write_width(key: &str, value: i32) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(key, &value.to_string());
    }
}
