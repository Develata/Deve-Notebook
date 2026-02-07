use super::model::{LineView, UnifiedLine, compute_diff, to_unified};
use super::unified::{ChunkWindow, slice_lines};
use gloo_timers::callback::Timeout;
use leptos::html;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DiffComputeState {
    pub is_editing: ReadSignal<bool>,
    pub set_is_editing: WriteSignal<bool>,
    pub content: ReadSignal<String>,
    pub set_content: WriteSignal<String>,
    pub compute_state: ReadSignal<String>,
    pub diff_result: Memo<(Vec<LineView>, Vec<LineView>)>,
    pub unified_lines: Memo<Vec<UnifiedLine>>,
}

pub struct UnifiedViewportState {
    pub unified_ref: NodeRef<html::Div>,
    pub set_scroll_top: WriteSignal<i32>,
    pub set_viewport_h: WriteSignal<i32>,
    pub window: Memo<ChunkWindow>,
    pub visible_unified: Memo<Vec<UnifiedLine>>,
}

pub fn create_compute_state(old_content: String, new_content: String) -> DiffComputeState {
    let (is_editing, set_is_editing) = signal(false);
    let (content, set_content) = signal(new_content.clone());
    let (debounced_content, set_debounced_content) = signal(new_content);
    let (compute_state, set_compute_state) = signal("ready".to_string());
    let (compute_token, set_compute_token) = signal(0u64);
    let timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));

    Effect::new({
        let timer = timer.clone();
        move |_| {
            let text = content.get();
            set_compute_state.set("computing".to_string());
            let next_token = compute_token.get_untracked().wrapping_add(1);
            set_compute_token.set(next_token);
            if let Some(t) = timer.borrow_mut().take() {
                t.cancel();
            }
            let latest = compute_token;
            let handle = Timeout::new(150, move || {
                if latest.get_untracked() == next_token {
                    set_debounced_content.set(text);
                    set_compute_state.set("ready".to_string());
                }
            });
            *timer.borrow_mut() = Some(handle);
        }
    });

    let diff_result = Memo::new(move |prev| {
        let _token = compute_token.get();
        if is_editing.get() {
            return prev
                .cloned()
                .unwrap_or_else(|| compute_diff(&old_content, &debounced_content.get()));
        }
        compute_diff(&old_content, &debounced_content.get())
    });

    let unified_lines = Memo::new(move |_| {
        let (left, right) = diff_result.get();
        to_unified(&left, &right)
    });

    DiffComputeState {
        is_editing,
        set_is_editing,
        content,
        set_content,
        compute_state,
        diff_result,
        unified_lines,
    }
}

pub fn create_unified_viewport(unified_lines: Memo<Vec<UnifiedLine>>) -> UnifiedViewportState {
    let unified_ref = NodeRef::<html::Div>::new();
    let (scroll_top, set_scroll_top) = signal(0i32);
    let (viewport_h, set_viewport_h) = signal(600i32);
    let window = Memo::new(move |_| {
        ChunkWindow::from_viewport(
            unified_lines.get().len(),
            scroll_top.get(),
            viewport_h.get(),
        )
    });
    let visible_unified = Memo::new(move |_| slice_lines(&unified_lines.get(), window.get()));

    UnifiedViewportState {
        unified_ref,
        set_scroll_top,
        set_viewport_h,
        window,
        visible_unified,
    }
}
