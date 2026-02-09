use super::fold::FoldState;
use super::model::hunk_fold::UnifiedRow;
use super::model::split_fold::SplitRow;
use super::split_pane::SplitPane;
use super::state::{ComputePhase, DiffComputeState};
use super::unified_pane::UnifiedPane;
use super::viewport::UnifiedViewportState;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

#[derive(Clone)]
pub struct DiffBodyDeps {
    pub force_unified: bool,
    pub locale: RwSignal<Locale>,
    pub compute: DiffComputeState,
    pub fold_controls: FoldState,
    pub fold_render: FoldState,
    pub folded_rows: Memo<Vec<UnifiedRow>>,
    pub split_rows: Memo<Vec<SplitRow>>,
    pub viewport: UnifiedViewportState,
    pub left_ref: NodeRef<html::Div>,
    pub right_ref: NodeRef<html::Div>,
    pub syncing_left: ReadSignal<bool>,
    pub set_syncing_left: WriteSignal<bool>,
    pub syncing_right: ReadSignal<bool>,
    pub set_syncing_right: WriteSignal<bool>,
}

#[component]
pub fn DiffBody(deps: DiffBodyDeps) -> impl IntoView {
    view! {
        <div class="flex-1 overflow-hidden flex relative">
            <Show when=move || deps.compute.compute_state.get() != ComputePhase::Ready>
                <div class="diff-compute-indicator absolute top-2 right-2 z-10 rounded border border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-2 py-1 text-[11px] text-[var(--diff-muted)] shadow-sm">
                    {move || t::diff::computing(deps.locale.get())}
                </div>
            </Show>
            <Show when=move || !deps.compute.is_editing.get()>
                <super::fold_controls::FoldControls fold_state=deps.fold_controls.clone() />
            </Show>

            {move || if deps.force_unified {
                if deps.compute.is_editing.get() {
                    view! { <div class="flex-1 overflow-auto"><textarea name="diff-edit-mobile" class="w-full h-full p-2 resize-none outline-none font-mono text-[13px] bg-[var(--diff-bg)] text-[var(--diff-fg)] border-none" prop:value=move || deps.compute.content.get() on:input=move |ev| deps.compute.set_content.set(event_target_value(&ev))></textarea></div> }.into_any()
                } else {
                    view! {
                        <UnifiedPane
                            lines=deps.folded_rows
                            visible_lines=deps.viewport.visible_unified
                            window=deps.viewport.window
                            unified_ref=deps.viewport.unified_ref
                            set_scroll_top=deps.viewport.set_scroll_top
                            set_viewport_h=deps.viewport.set_viewport_h
                            compute_state=deps.compute.compute_state
                            on_expand_fold=deps.fold_render.on_expand_fold
                        />
                    }.into_any()
                }
            } else {
                view! {
                    <SplitPane
                        split_rows=deps.split_rows
                        left_ref=deps.left_ref
                        right_ref=deps.right_ref
                        syncing_left=deps.syncing_left
                        set_syncing_left=deps.set_syncing_left
                        syncing_right=deps.syncing_right
                        set_syncing_right=deps.set_syncing_right
                        is_editing=deps.compute.is_editing
                        content=deps.compute.content
                        set_content=deps.compute.set_content
                        on_expand_fold=deps.fold_render.on_expand_fold
                    />
                }.into_any()
            }}
        </div>
    }
}
