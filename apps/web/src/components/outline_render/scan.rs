// apps/web/src/components/outline_render/scan.rs
//! plan_ref:
//!   - 10_rendering#markdown-render-whitelist
//!   - 10_rendering#document-authority-bridge
//!
//! Shared inline scan helpers for outline parsing.

pub(super) fn next_char_at(text: &str, index: usize) -> Option<(char, usize)> {
    let ch = text.get(index..)?.chars().next()?;
    Some((ch, ch.len_utf8()))
}

pub(super) fn tail_starts_with(text: &str, index: usize, marker: &str) -> bool {
    text.get(index..)
        .is_some_and(|tail| tail.starts_with(marker))
}

pub fn find_next_char(text: &str, start: usize, target: char) -> Option<usize> {
    let mut i = start;
    while let Some((ch, len)) = next_char_at(text, i) {
        if ch == target {
            return Some(i);
        }
        i += len;
    }
    None
}

pub fn find_math_close(text: &str, start: usize) -> Option<usize> {
    let mut i = start;
    while let Some((ch, len)) = next_char_at(text, i) {
        if ch == '\\' {
            i += len;
            if let Some((_, next_len)) = next_char_at(text, i) {
                i += next_len;
            }
            continue;
        }
        if ch == '$' {
            return Some(i);
        }
        i += len;
    }
    None
}

pub fn find_style_close(text: &str, start: usize, marker: &str) -> Option<usize> {
    let mut i = start;
    while let Some((ch, len)) = next_char_at(text, i) {
        if ch == '\\' {
            i += len;
            if let Some((_, next_len)) = next_char_at(text, i) {
                i += next_len;
            }
            continue;
        }
        if tail_starts_with(text, i, marker) {
            return Some(i);
        }
        i += len;
    }
    None
}
