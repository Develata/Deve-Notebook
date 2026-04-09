// apps/web/src/components/outline_render/scan.rs
//! Shared inline scan helpers for outline parsing.

pub fn find_next_char(text: &str, start: usize, target: char) -> Option<usize> {
    let mut i = start;
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        if ch == target {
            return Some(i);
        }
        i += ch.len_utf8();
    }
    None
}

pub fn find_math_close(text: &str, start: usize) -> Option<usize> {
    let mut i = start;
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        let len = ch.len_utf8();
        if ch == '\\' {
            i += len;
            if i < text.len() {
                let next_len = text[i..].chars().next().unwrap().len_utf8();
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
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        let len = ch.len_utf8();
        if ch == '\\' {
            i += len;
            if i < text.len() {
                let next_len = text[i..].chars().next().unwrap().len_utf8();
                i += next_len;
            }
            continue;
        }
        if text[i..].starts_with(marker) {
            return Some(i);
        }
        i += len;
    }
    None
}
