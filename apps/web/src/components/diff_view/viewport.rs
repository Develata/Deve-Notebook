//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#large-document-runtime
//!
use super::model::hunk_fold::UnifiedRow;
use super::unified::{ChunkWindow, slice_lines};
use leptos::html;
use leptos::prelude::*;

pub const DEFAULT_DIFF_VIEWPORT_HEIGHT_PX: i32 = 600;

pub fn initial_diff_viewport_height_px() -> i32 {
    DEFAULT_DIFF_VIEWPORT_HEIGHT_PX
}

#[derive(Clone)]
pub struct UnifiedViewportState {
    pub unified_ref: NodeRef<html::Div>,
    pub set_scroll_top: WriteSignal<i32>,
    pub set_viewport_h: WriteSignal<i32>,
    pub window: Memo<ChunkWindow>,
    pub visible_unified: Memo<Vec<UnifiedRow>>,
}

pub fn create_unified_viewport(unified_lines: Memo<Vec<UnifiedRow>>) -> UnifiedViewportState {
    let unified_ref = NodeRef::<html::Div>::new();
    let (scroll_top, set_scroll_top) = signal(0i32);
    let (viewport_h, set_viewport_h) = signal(initial_diff_viewport_height_px());
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

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DIFF_VIEWPORT_HEIGHT_PX, initial_diff_viewport_height_px};

    #[test]
    fn diff_first_viewport_default_height_is_stable() {
        assert_eq!(
            initial_diff_viewport_height_px(),
            DEFAULT_DIFF_VIEWPORT_HEIGHT_PX
        );
        assert!(initial_diff_viewport_height_px() > 0);
    }
}
