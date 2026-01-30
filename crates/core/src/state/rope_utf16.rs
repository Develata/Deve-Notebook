use ropey::Rope;

pub(super) fn utf16_to_char_idx(rope: &Rope, utf16_index: usize) -> usize {
    if utf16_index == 0 {
        return 0;
    }

    let mut remaining = utf16_index;
    let mut char_idx = 0usize;

    for chunk in rope.chunks() {
        let chunk_utf16 = chunk.encode_utf16().count();
        if remaining > chunk_utf16 {
            remaining -= chunk_utf16;
            char_idx += chunk.chars().count();
            if remaining == 0 {
                return char_idx;
            }
            continue;
        }

        for ch in chunk.chars() {
            let next = ch.len_utf16();
            if remaining <= next {
                return char_idx + 1;
            }
            remaining -= next;
            char_idx += 1;
        }
        return char_idx;
    }

    rope.len_chars()
}
