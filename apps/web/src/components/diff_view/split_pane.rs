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
            <div class="flex-1 flex overflow-auto border-r border-[var(--diff-border)]" node_ref=left_ref on:scroll=move |_| {
                if syncing_right.get() { return; }
                if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                    set_syncing_left.set(true);
                    right.set_scroll_top(left.scroll_top());
                    set_syncing_left.set(false);
                }
            }>
                <SplitLeftColumn split_rows=split_rows on_expand_fold=on_expand_fold />
            </div>

            <div class="flex-1 flex overflow-auto relative" node_ref=right_ref on:scroll=move |_| {
                if syncing_left.get() { return; }
                if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                    set_syncing_right.set(true);
                    left.set_scroll_top(right.scroll_top());
                    set_syncing_right.set(false);
                }
            }>
                <SplitRightColumn split_rows=split_rows is_editing=is_editing content=content set_content=set_content on_expand_fold=on_expand_fold />
            </div>
        </>
    }
}
