// apps/web/src/components/outline_render.rs
//! # Outline Inline Renderer
//!
//! Render inline math/code for outline items.

use js_sys::{Function, Object, Reflect};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentKind {
    Text,
    Code,
    Math,
}

#[derive(Clone, Debug)]
struct Segment {
    kind: SegmentKind,
    text: String,
}

pub fn render_outline_inline(text: &str) -> Vec<AnyView> {
    let segments = split_inline_segments(text);
    segments
        .into_iter()
        .map(|seg| match seg.kind {
            SegmentKind::Text => view! { <span>{seg.text}</span> }.into_any(),
            SegmentKind::Code => {
                view! { <span class="cm-inline-code">{seg.text}</span> }.into_any()
            }
            SegmentKind::Math => match render_katex_to_string(&seg.text) {
                Some(html) => {
                    view! { <span class="cm-math-widget" inner_html=html></span> }.into_any()
                }
                None => view! { <span>{format!("${}$", seg.text)}</span> }.into_any(),
            },
        })
        .collect()
}

fn split_inline_segments(text: &str) -> Vec<Segment> {
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

fn render_katex_to_string(expr: &str) -> Option<String> {
    let window = web_sys::window()?;
    let katex = Reflect::get(&window, &JsValue::from_str("katex")).ok()?;
    if katex.is_undefined() {
        return None;
    }
    let render = Reflect::get(&katex, &JsValue::from_str("renderToString")).ok()?;
    let func: Function = render.dyn_into().ok()?;
    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("throwOnError"),
        &JsValue::FALSE,
    );
    let _ = Reflect::set(&options, &JsValue::from_str("displayMode"), &JsValue::FALSE);
    let html = func
        .call2(&katex, &JsValue::from_str(expr), &options)
        .ok()?;
    html.as_string()
}
