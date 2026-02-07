use super::model::{LINE_HEIGHT_PX, LineKind, UnifiedLine};
use super::unified::hunk_rows as collect_hunks;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

pub struct HunkNavState {
    pub has_hunks: Signal<bool>,
    pub hunk_index_text: Signal<String>,
    pub added_count: Signal<usize>,
    pub deleted_count: Signal<usize>,
    pub on_prev_hunk: Callback<()>,
    pub on_next_hunk: Callback<()>,
}

pub fn create_hunk_nav(
    unified_lines: Memo<Vec<UnifiedLine>>,
    force_unified: bool,
    unified_ref: NodeRef<html::Div>,
    left_ref: NodeRef<html::Div>,
    right_ref: NodeRef<html::Div>,
) -> HunkNavState {
    let hunk_rows = Memo::new(move |_| collect_hunks(&unified_lines.get()));
    let (hunk_idx, set_hunk_idx) = signal(0usize);
    let has_hunks = Signal::derive(move || !hunk_rows.get().is_empty());
    let hunk_index_text = Signal::derive(move || {
        let rows = hunk_rows.get();
        if rows.is_empty() {
            return "0/0".to_string();
        }
        let idx = hunk_idx.get().min(rows.len().saturating_sub(1));
        format!("{}/{}", idx + 1, rows.len())
    });
    let added_count = Signal::derive(move || {
        unified_lines
            .get()
            .into_iter()
            .filter(|l| l.kind == LineKind::Add)
            .count()
    });
    let deleted_count = Signal::derive(move || {
        unified_lines
            .get()
            .into_iter()
            .filter(|l| l.kind == LineKind::Del)
            .count()
    });

    Effect::new(move |_| {
        let count = hunk_rows.get().len();
        if count == 0 {
            set_hunk_idx.set(0);
        } else if hunk_idx.get() >= count {
            set_hunk_idx.set(count - 1);
        }
    });

    let jump_to_hunk = Callback::new(move |idx: usize| {
        let rows = hunk_rows.get_untracked();
        if rows.is_empty() {
            return;
        }
        let target_idx = idx % rows.len();
        set_hunk_idx.set(target_idx);
        let top = (rows[target_idx] as i32) * LINE_HEIGHT_PX;
        if force_unified {
            if let Some(el) = unified_ref.get_untracked() {
                el.set_scroll_top(top);
            }
        } else {
            if let Some(left) = left_ref.get_untracked() {
                left.set_scroll_top(top);
            }
            if let Some(right) = right_ref.get_untracked() {
                right.set_scroll_top(top);
            }
        }
    });

    let on_prev_hunk = Callback::new(move |_| {
        let count = hunk_rows.get_untracked().len();
        if count == 0 {
            return;
        }
        let current = hunk_idx.get_untracked();
        jump_to_hunk.run((current + count - 1) % count);
    });
    let on_next_hunk = Callback::new(move |_| {
        let count = hunk_rows.get_untracked().len();
        if count == 0 {
            return;
        }
        let current = hunk_idx.get_untracked();
        jump_to_hunk.run((current + 1) % count);
    });

    HunkNavState {
        has_hunks,
        hunk_index_text,
        added_count,
        deleted_count,
        on_prev_hunk,
        on_next_hunk,
    }
}

pub fn should_ignore_shortcut(ev: &web_sys::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(el) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    let tag = el.tag_name().to_ascii_lowercase();
    tag == "input" || tag == "textarea" || el.has_attribute("contenteditable")
}
