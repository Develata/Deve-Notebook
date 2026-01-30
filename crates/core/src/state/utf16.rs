pub(super) fn utf16_len(text: &str) -> Option<u32> {
    u32::try_from(text.encode_utf16().count()).ok()
}

pub(super) fn add_utf16_pos(pos: &mut u32, text: &str) -> bool {
    let delta = match utf16_len(text) {
        Some(v) => v,
        None => return false,
    };
    match pos.checked_add(delta) {
        Some(next) => {
            *pos = next;
            true
        }
        None => false,
    }
}
