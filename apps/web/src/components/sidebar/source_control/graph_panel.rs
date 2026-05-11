//! plan_ref:
//!   - 14_tech_stack#graph-visualization
//!   - 07_diff_logic#source-control-runtime
//!
//! Minimal read-only graph projection summary. Renderer work remains future.

use super::status_notice::{blocked_hint, blocked_title};
use crate::api::{GraphProjectionFetchError, fetch_graph_projection};
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use deve_core::graph::GraphProjection;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Debug, PartialEq)]
enum GraphProjectionFetchState {
    Idle,
    Loading,
    Loaded(GraphProjection),
    Error,
    Blocked(RepoWriteBlock),
    LocalOnly,
    Degraded,
}

#[component]
pub fn GraphPanel(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let fetch_state = RwSignal::new(GraphProjectionFetchState::Idle);

    Effect::new(move |_| {
        if !expanded.get() {
            fetch_state.set(GraphProjectionFetchState::Idle);
            return;
        }
        let repo_id = core.current_repo_id.get();
        if repo_id.is_none() {
            fetch_state.set(GraphProjectionFetchState::Idle);
            return;
        }
        if core.active_branch.get().is_some() {
            fetch_state.set(GraphProjectionFetchState::LocalOnly);
            return;
        }
        if let Some(block) = core.read_block.get() {
            fetch_state.set(GraphProjectionFetchState::Blocked(block));
            return;
        }

        fetch_state.set(GraphProjectionFetchState::Loading);
        spawn_local(async move {
            let fetched = fetch_graph_projection(repo_id.clone()).await;
            let still_current = expanded.get_untracked()
                && core.current_repo_id.get_untracked() == repo_id
                && core.active_branch.get_untracked().is_none()
                && core.read_block.get_untracked().is_none();
            if still_current {
                fetch_state.set(match fetched {
                    Ok(projection) => GraphProjectionFetchState::Loaded(projection),
                    Err(GraphProjectionFetchError::DegradedProjectionRequired) => {
                        GraphProjectionFetchState::Degraded
                    }
                    Err(GraphProjectionFetchError::RequestFailed) => {
                        GraphProjectionFetchState::Error
                    }
                });
            }
        });
    });

    view! {
        <div class="border-t border-default">
            <button
                class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase"
                on:click=move |_| expanded.update(|open| *open = !*open)
            >
                <span class=move || if expanded.get() {
                    "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform"
                } else {
                    "w-4 h-4 flex items-center justify-center transition-transform"
                }>
                    <ChevronRight class="w-3 h-3" />
                </span>
                <span class="flex-1 text-left">{move || t::source_control::graph(locale.get())}</span>
            </button>
            <Show when=move || expanded.get()>
                <div class="px-4 pb-3 pt-2" data-deve-graph-panel="readonly">
                    {move || graph_panel_body(locale.get(), &fetch_state.get())}
                </div>
            </Show>
        </div>
    }
}

fn graph_panel_body(locale: Locale, state: &GraphProjectionFetchState) -> AnyView {
    match state {
        GraphProjectionFetchState::Idle => graph_message(
            "idle",
            t::source_control::graph_projection_empty(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Loading => graph_message(
            "loading",
            t::source_control::loading_graph(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Error => graph_message(
            "error",
            t::source_control::graph_projection_unavailable(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Blocked(block) => graph_message(
            "blocked",
            t::source_control::graph_projection_blocked(locale),
            graph_blocked_note(locale, *block),
        ),
        GraphProjectionFetchState::LocalOnly => graph_message(
            "local-only",
            t::source_control::graph_projection_local_only(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Degraded => graph_message(
            "degraded",
            t::source_control::graph_projection_degraded(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Loaded(projection) => {
            graph_projection_summary(locale, projection)
        }
    }
}

fn graph_blocked_note(locale: Locale, block: RepoWriteBlock) -> String {
    format!(
        "{}: {}",
        blocked_title(locale, block),
        blocked_hint(locale, block)
    )
}

fn graph_loaded_state_attr(projection: &GraphProjection) -> &'static str {
    if projection.nodes.is_empty() {
        "empty"
    } else {
        "loaded"
    }
}

fn graph_message(
    state: &'static str,
    message: impl Into<String>,
    note: impl Into<String>,
) -> AnyView {
    let message = message.into();
    let note = note.into();
    view! {
        <div data-deve-graph-state=state>
            <p class="text-[12px] text-muted">{message}</p>
            <p class="mt-2 border-l border-default pl-2 text-[11px] text-muted">{note}</p>
        </div>
    }
    .into_any()
}

fn graph_projection_summary(locale: Locale, projection: &GraphProjection) -> AnyView {
    let nodes = projection.nodes.len();
    let edges = projection.edges.len();
    let unresolved = projection.unresolved_links.len();
    let state = graph_loaded_state_attr(projection);
    view! {
        <div data-deve-graph-state=state>
            <div class="grid grid-cols-3 gap-2">
                <GraphStat
                    label=t::source_control::graph_nodes(locale)
                    value=nodes
                    attr="nodes"
                />
                <GraphStat
                    label=t::source_control::graph_edges(locale)
                    value=edges
                    attr="edges"
                />
                <GraphStat
                    label=t::source_control::graph_unresolved_links(locale)
                    value=unresolved
                    attr="unresolved"
                />
            </div>
            <p class="mt-2 border-l border-default pl-2 text-[11px] text-muted">
                {t::source_control::graph_readonly_note(locale)}
            </p>
        </div>
    }
    .into_any()
}

#[component]
fn GraphStat(label: &'static str, value: usize, attr: &'static str) -> impl IntoView {
    view! {
        <div
            class="rounded border border-default bg-panel/70 px-2 py-1"
            data-deve-graph-stat=attr
        >
            <p class="text-[10px] uppercase tracking-wide text-muted">{label}</p>
            <p class="mt-0.5 font-mono text-[15px] text-primary">{value}</p>
        </div>
    }
}

#[cfg(test)]
mod tests;
