//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::model::split_fold::SplitRow;
use super::split_columns::{SplitLeftColumn, SplitRightColumn};
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn SplitPane(
    split_rows: Memo<Vec<SplitRow>>,
    left_ref: NodeRef<html::Div>,
    right_ref: NodeRef<html::Div>,
    syncing_left: ReadSignal<bool>,
    set_syncing_left: WriteSignal<bool>,
    syncing_right: ReadSignal<bool>,
    set_syncing_right: WriteSignal<bool>,
    is_editing: ReadSignal<bool>,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
    on_expand_fold: Callback<usize>,
) -> impl IntoView {
    view! {
        <>
            <div
                data-deve-desktop-col="2-diff-old"
                data-deve-diff-scroll-sync="source-left"
                class="flex-1 flex overflow-auto border-r border-[var(--diff-border)]"
                node_ref=left_ref
                on:scroll=move |_| {
                if syncing_right.get() {
                    set_syncing_right.set(false);
                    return;
                }
                if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                    let target_top = synced_scroll_top(
                        left.scroll_top(),
                        left.scroll_height(),
                        left.client_height(),
                        right.scroll_height(),
                        right.client_height(),
                    );
                    if needs_scroll_sync(right.scroll_top(), target_top) {
                        set_syncing_left.set(true);
                        right.set_scroll_top(target_top);
                    }
                }
            }>
                <SplitLeftColumn split_rows=split_rows on_expand_fold=on_expand_fold />
            </div>

            <div
                data-deve-desktop-col="3-editor"
                data-deve-diff-scroll-sync="source-right"
                class="flex-1 flex overflow-auto relative"
                node_ref=right_ref
                on:scroll=move |_| {
                if syncing_left.get() {
                    set_syncing_left.set(false);
                    return;
                }
                if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                    let target_top = synced_scroll_top(
                        right.scroll_top(),
                        right.scroll_height(),
                        right.client_height(),
                        left.scroll_height(),
                        left.client_height(),
                    );
                    if needs_scroll_sync(left.scroll_top(), target_top) {
                        set_syncing_right.set(true);
                        left.set_scroll_top(target_top);
                    }
                }
            }>
                <SplitRightColumn split_rows=split_rows is_editing=is_editing content=content set_content=set_content on_expand_fold=on_expand_fold />
            </div>
        </>
    }
}

fn synced_scroll_top(
    source_top: i32,
    source_scroll_height: i32,
    source_client_height: i32,
    target_scroll_height: i32,
    target_client_height: i32,
) -> i32 {
    let source_max = scroll_max(source_scroll_height, source_client_height);
    let target_max = scroll_max(target_scroll_height, target_client_height);
    if source_max == 0 || target_max == 0 {
        return 0;
    }
    let source_top = source_top.clamp(0, source_max);
    ((source_top as f64 / source_max as f64) * target_max as f64)
        .round()
        .clamp(0.0, target_max as f64) as i32
}

fn scroll_max(scroll_height: i32, client_height: i32) -> i32 {
    scroll_height.saturating_sub(client_height).max(0)
}

fn needs_scroll_sync(current_top: i32, target_top: i32) -> bool {
    current_top != target_top
}

#[cfg(test)]
mod tests {
    use super::{needs_scroll_sync, scroll_max, synced_scroll_top};

    #[test]
    fn desktop_diff_scroll_syncs_col3_to_col2_by_ratio() {
        assert_eq!(synced_scroll_top(300, 1200, 600, 2400, 600), 900);
    }

    #[test]
    fn desktop_diff_scroll_sync_clamps_source_overflow() {
        assert_eq!(synced_scroll_top(900, 1200, 600, 2400, 600), 1800);
    }

    #[test]
    fn desktop_diff_scroll_sync_handles_non_scrollable_pane() {
        assert_eq!(synced_scroll_top(300, 600, 600, 2400, 600), 0);
        assert_eq!(scroll_max(500, 700), 0);
    }

    #[test]
    fn desktop_diff_scroll_sync_skips_noop_target_update() {
        assert!(!needs_scroll_sync(300, 300));
        assert!(needs_scroll_sync(299, 300));
    }
}
