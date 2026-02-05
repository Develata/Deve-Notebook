// apps/web/src/components/outline_render/parse.rs
//! # Outline Inline Parser

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    Text,
    Code,
    Math,
    Strong,
    Em,
    Del,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub kind: SegmentKind,
    pub text: String,
}

pub fn split_inline_segments(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut last = 0;
    let mut i = 0;

    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        let len = ch.len_utf8();

        if ch == '\\' && i + len < text.len() {
            i += len;
            let next_len = text[i..].chars().next().unwrap().len_utf8();
            i += next_len;
            continue;
        }

        if ch == '`'
            && let Some(close) = find_next_char(text, i + len, '`')
        {
            push_text(&mut segments, text, last, i);
            let code = &text[i + len..close];
            segments.push(Segment {
                kind: SegmentKind::Code,
                text: code.to_string(),
            });
            i = close + len;
            last = i;
            continue;
        }

        if ch == '$'
            && i + len < text.len()
            && let Some(next) = text[i + len..].chars().next()
            && !next.is_whitespace()
            && let Some(close) = find_math_close(text, i + len)
        {
            push_text(&mut segments, text, last, i);
            let math = &text[i + len..close];
            segments.push(Segment {
                kind: SegmentKind::Math,
                text: math.to_string(),
            });
            i = close + len;
            last = i;
            continue;
        }

        if ch == '*' || ch == '~' {
            let (marker, kind) = if ch == '*' && text[i + len..].starts_with('*') {
                ("**", SegmentKind::Strong)
            } else if ch == '~' && text[i + len..].starts_with('~') {
                ("~~", SegmentKind::Del)
            } else if ch == '*' {
                ("*", SegmentKind::Em)
            } else {
                ("", SegmentKind::Text)
            };

            if !marker.is_empty()
                && let Some(close) = find_style_close(text, i + marker.len(), marker)
            {
                push_text(&mut segments, text, last, i);
                let inner = &text[i + marker.len()..close];
                segments.push(Segment {
                    kind,
                    text: inner.to_string(),
                });
                i = close + marker.len();
                last = i;
                continue;
            }
        }

        i += len;
    }

    push_text(&mut segments, text, last, text.len());
    segments
}

fn push_text(segments: &mut Vec<Segment>, text: &str, start: usize, end: usize) {
    if end > start {
        segments.push(Segment {
            kind: SegmentKind::Text,
            text: text[start..end].to_string(),
        });
    }
}

fn find_next_char(text: &str, start: usize, target: char) -> Option<usize> {
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

fn find_math_close(text: &str, start: usize) -> Option<usize> {
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

fn find_style_close(text: &str, start: usize, marker: &str) -> Option<usize> {
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
