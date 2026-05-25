// apps/web/src/components/outline_render/parse.rs
//! plan_ref:
//!   - 10_rendering#markdown-render-whitelist
//!   - 10_rendering#document-authority-bridge
//!
//! # Outline Inline Parser

use super::scan::{
    find_math_close, find_next_char, find_style_close, next_char_at, tail_starts_with,
};

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

    while let Some((ch, len)) = next_char_at(text, i) {
        if ch == '\\' {
            if let Some((_, next_len)) = next_char_at(text, i + len) {
                i += len + next_len;
            } else {
                i += len;
            }
            continue;
        }

        if ch == '`'
            && let Some(close) = find_next_char(text, i + len, '`')
        {
            push_text(&mut segments, text, last, i);
            if let Some(code) = text.get(i + len..close) {
                segments.push(Segment {
                    kind: SegmentKind::Code,
                    text: code.to_string(),
                });
                i = close + len;
                last = i;
                continue;
            }
        }

        if ch == '$'
            && let Some((next, _)) = next_char_at(text, i + len)
            && !next.is_whitespace()
            && let Some(close) = find_math_close(text, i + len)
        {
            push_text(&mut segments, text, last, i);
            if let Some(math) = text.get(i + len..close) {
                segments.push(Segment {
                    kind: SegmentKind::Math,
                    text: math.to_string(),
                });
                i = close + len;
                last = i;
                continue;
            }
        }

        if ch == '*' || ch == '~' || ch == '_' {
            let (marker, kind) = if ch == '*' && tail_starts_with(text, i + len, "*") {
                ("**", SegmentKind::Strong)
            } else if ch == '_' && tail_starts_with(text, i + len, "_") {
                ("__", SegmentKind::Strong)
            } else if ch == '~' && tail_starts_with(text, i + len, "~") {
                ("~~", SegmentKind::Del)
            } else if ch == '*' {
                ("*", SegmentKind::Em)
            } else if ch == '_' {
                ("_", SegmentKind::Em)
            } else {
                ("", SegmentKind::Text)
            };

            if !marker.is_empty()
                && let Some(close) = find_style_close(text, i + marker.len(), marker)
            {
                push_text(&mut segments, text, last, i);
                if let Some(inner) = text.get(i + marker.len()..close) {
                    segments.push(Segment {
                        kind,
                        text: inner.to_string(),
                    });
                    i = close + marker.len();
                    last = i;
                    continue;
                }
            }
        }

        i += len;
    }

    push_text(&mut segments, text, last, text.len());
    segments
}

fn push_text(segments: &mut Vec<Segment>, text: &str, start: usize, end: usize) {
    if end > start
        && let Some(text) = text.get(start..end)
    {
        segments.push(Segment {
            kind: SegmentKind::Text,
            text: text.to_string(),
        });
    }
}
