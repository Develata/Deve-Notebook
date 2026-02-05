// apps/web/src/components/outline_render/mod.rs
//! # Outline Inline Renderer
//!
//! Render inline math/code/styles for outline items.

use leptos::prelude::*;

mod katex;
mod parse;

pub fn render_outline_inline(text: &str) -> Vec<AnyView> {
    let segments = parse::split_inline_segments(text);
    segments
        .into_iter()
        .map(|seg| match seg.kind {
            parse::SegmentKind::Text => view! { <span>{seg.text}</span> }.into_any(),
            parse::SegmentKind::Code => {
                view! { <span class="cm-inline-code">{seg.text}</span> }.into_any()
            }
            parse::SegmentKind::Math => match katex::render_katex_to_string(&seg.text) {
                Some(html) => {
                    view! { <span class="cm-math-widget" inner_html=html></span> }.into_any()
                }
                None => view! { <span>{format!("${}$", seg.text)}</span> }.into_any(),
            },
            parse::SegmentKind::Strong => view! {
                <strong>{render_outline_inline(&seg.text)}</strong>
            }
            .into_any(),
            parse::SegmentKind::Em => view! {
                <em>{render_outline_inline(&seg.text)}</em>
            }
            .into_any(),
            parse::SegmentKind::Del => view! {
                <del>{render_outline_inline(&seg.text)}</del>
            }
            .into_any(),
            parse::SegmentKind::Mark => view! {
                <mark>{render_outline_inline(&seg.text)}</mark>
            }
            .into_any(),
        })
        .collect()
}
