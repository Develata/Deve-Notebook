//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
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

pub fn format_hunk_index_text(current: usize, count: usize) -> String {
    if count == 0 {
        return "0/0".to_string();
    }
    let idx = current.min(count.saturating_sub(1));
    format!("{}/{}", idx + 1, count)
}

pub fn next_hunk_index(current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some((current + 1) % count)
}

pub fn prev_hunk_index(current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some((current + count - 1) % count)
}

pub fn count_lines_by_kind(lines: &[UnifiedLine], kind: LineKind) -> usize {
    lines.iter().filter(|line| line.kind == kind).count()
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
    let hunk_index_text =
        Signal::derive(move || format_hunk_index_text(hunk_idx.get(), hunk_rows.get().len()));
    let added_count =
        Signal::derive(move || count_lines_by_kind(&unified_lines.get(), LineKind::Add));
    let deleted_count =
        Signal::derive(move || count_lines_by_kind(&unified_lines.get(), LineKind::Del));

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
        if let Some(idx) = prev_hunk_index(hunk_idx.get_untracked(), count) {
            jump_to_hunk.run(idx);
        }
    });
    let on_next_hunk = Callback::new(move |_| {
        let count = hunk_rows.get_untracked().len();
        if let Some(idx) = next_hunk_index(hunk_idx.get_untracked(), count) {
            jump_to_hunk.run(idx);
        }
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

#[cfg(test)]
mod tests {
    use super::{count_lines_by_kind, format_hunk_index_text, next_hunk_index, prev_hunk_index};
    use crate::components::diff_view::model::{LineKind, UnifiedLine};

    fn line(kind: LineKind) -> UnifiedLine {
        UnifiedLine {
            num: Some(1),
            content: String::new(),
            class: "",
            word_ranges: Vec::new(),
            kind,
        }
    }

    #[test]
    fn diff_hunk_navigation_indices_wrap() {
        assert_eq!(next_hunk_index(0, 3), Some(1));
        assert_eq!(next_hunk_index(2, 3), Some(0));
        assert_eq!(prev_hunk_index(0, 3), Some(2));
        assert_eq!(prev_hunk_index(2, 3), Some(1));
        assert_eq!(next_hunk_index(0, 0), None);
        assert_eq!(prev_hunk_index(0, 0), None);
    }

    #[test]
    fn diff_hunk_index_text_clamps_to_available_hunks() {
        assert_eq!(format_hunk_index_text(0, 0), "0/0");
        assert_eq!(format_hunk_index_text(0, 3), "1/3");
        assert_eq!(format_hunk_index_text(9, 3), "3/3");
    }

    #[test]
    fn diff_header_change_stats_count_added_and_deleted_lines() {
        let lines = vec![
            line(LineKind::Normal),
            line(LineKind::Add),
            line(LineKind::Del),
            line(LineKind::Add),
        ];

        assert_eq!(count_lines_by_kind(&lines, LineKind::Add), 2);
        assert_eq!(count_lines_by_kind(&lines, LineKind::Del), 1);
    }
}
