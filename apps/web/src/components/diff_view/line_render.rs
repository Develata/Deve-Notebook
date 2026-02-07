use super::model::LineKind;
use leptos::prelude::*;

fn class_for(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Add => "diff-word-mark bg-[var(--diff-word-add)]",
        LineKind::Del => "diff-word-mark bg-[var(--diff-word-del)]",
        _ => "",
    }
}

#[component]
pub fn LineRender(content: String, ranges: Vec<(usize, usize)>, kind: LineKind) -> impl IntoView {
    if ranges.is_empty() || content.is_empty() {
        return view! { <>{content}</> }.into_any();
    }

    let mut parts: Vec<(String, bool)> = Vec::new();
    let mut cursor = 0usize;
    for (start, end) in ranges {
        let s = start.min(content.len());
        let e = end.min(content.len());
        if s > cursor {
            parts.push((content[cursor..s].to_string(), false));
        }
        if e > s {
            parts.push((content[s..e].to_string(), true));
        }
        cursor = e;
    }
    if cursor < content.len() {
        parts.push((content[cursor..].to_string(), false));
    }

    let mark_class = class_for(kind);
    view! {
        <For
            each=move || parts.clone()
            key=|p| format!("{}{}", p.0, p.1 as u8)
            children=move |(text, highlighted)| {
                if highlighted {
                    view! { <span class=mark_class>{text}</span> }.into_any()
                } else {
                    view! { <span>{text}</span> }.into_any()
                }
            }
        />
    }
    .into_any()
}
