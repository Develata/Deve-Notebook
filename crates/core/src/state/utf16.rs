/// 将 UTF-16 code unit 索引转换为字节索引
///
/// **参数**:
/// * `s`: 原始字符串
/// * `utf16_index`: UTF-16 code unit 索引（与 JS/CodeMirror 一致）
///
/// **返回值**:
/// 对应的字节索引。如果索引超出范围，返回字符串的字节长度。
pub(super) fn utf16_to_byte_index(s: &str, utf16_index: usize) -> usize {
    if utf16_index == 0 {
        return 0;
    }

    let mut count = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if count >= utf16_index {
            return byte_idx;
        }
        let next = count + ch.len_utf16();
        if next >= utf16_index {
            return byte_idx + ch.len_utf8();
        }
        count = next;
    }
    s.len()
}

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
