mod anchor;
mod body;
mod cache;
mod fold;
mod fold_controls;
pub mod header;
mod lifecycle;
mod line_render;
mod metrics;
pub mod model;
mod navigation;
mod split_columns;
mod split_pane;
mod state;
pub mod unified;
mod unified_pane;
mod viewport;

use self::body::{DiffBody, DiffBodyDeps};
use self::header::DiffHeader;
use self::lifecycle::{setup_anchor_effects, setup_shortcuts};
use self::model::hunk_fold::build_folded_rows;
use self::model::split_fold::build_folded_split_rows;
use self::navigation::create_hunk_nav;
use self::state::create_compute_state;
use self::viewport::create_unified_viewport;
use crate::i18n::Locale;
use fold::create_fold_state;
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn DiffView(
    repo_scope: String,
    path: String,
    old_content: String,
    new_content: String,
    #[prop(default = false)] is_readonly: bool,
    #[prop(default = false)] force_unified: bool,
    #[prop(default = false)] mobile: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let filename = path
        .replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or("?")
        .to_string();

    let compute = create_compute_state(
        repo_scope,
        path.clone(),
        if force_unified { "unified" } else { "split" },
        5,
        old_content,
        new_content,
    );
    let left_ref = NodeRef::<html::Div>::new();
    let right_ref = NodeRef::<html::Div>::new();
    let (syncing_left, set_syncing_left) = signal(false);
    let (syncing_right, set_syncing_right) = signal(false);
    let fold_state = create_fold_state();
    Effect::new(move |_| {
        let _ = compute.unified_lines.get();
        fold_state.clear_expanded.run(());
    });
    let folded_rows = Memo::new(move |_| {
        build_folded_rows(
            &compute.unified_lines.get(),
            fold_state.context_lines.get(),
            fold_state.folding_enabled.get(),
            &fold_state.expanded_folds.get(),
        )
    });
    let split_rows = Memo::new(move |_| {
        let (left, right) = compute.diff_result.get();
        build_folded_split_rows(
            &left,
            &right,
            fold_state.context_lines.get(),
            fold_state.folding_enabled.get(),
            &fold_state.expanded_folds.get(),
        )
    });
    let viewport = create_unified_viewport(folded_rows);
    let nav = create_hunk_nav(
        compute.unified_lines,
        force_unified,
        viewport.unified_ref,
        left_ref,
        right_ref,
    );

    let fold_for_controls = fold_state.clone();
    let fold_for_render = fold_state.clone();
    let cache_hit = Signal::derive(move || compute.metrics.cache_hit.get());
    let cache_hit_ratio = Signal::derive(move || compute.metrics.cache_hit_ratio.get());
    let compute_ms = Signal::derive(move || compute.metrics.last_compute_ms.get());
    let algorithm = Signal::derive(move || compute.metrics.algorithm.get());
    setup_anchor_effects(
        force_unified,
        compute.compute_state,
        viewport.unified_ref,
        left_ref,
    );
    setup_shortcuts(on_close, nav.on_prev_hunk, nav.on_next_hunk);

    view! {
        <div class=move || if mobile { "diff-view-mobile h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]" } else { "h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]" }>
            <DiffHeader mobile=mobile filename=filename is_readonly=is_readonly is_editing=compute.is_editing hunk_index_text=nav.hunk_index_text has_hunks=nav.has_hunks added_count=nav.added_count deleted_count=nav.deleted_count cache_hit=cache_hit cache_hit_ratio=cache_hit_ratio compute_ms=compute_ms algorithm=algorithm on_prev_hunk=nav.on_prev_hunk on_next_hunk=nav.on_next_hunk toggle_edit=Callback::new(move |_| compute.set_is_editing.update(|v| *v = !*v)) on_close=on_close />
            <DiffBody deps=DiffBodyDeps {
                force_unified,
                locale,
                compute,
                fold_controls: fold_for_controls,
                fold_render: fold_for_render,
                folded_rows,
                split_rows,
                viewport,
                left_ref,
                right_ref,
                syncing_left,
                set_syncing_left,
                syncing_right,
                set_syncing_right,
            } />
        </div>
    }
}
