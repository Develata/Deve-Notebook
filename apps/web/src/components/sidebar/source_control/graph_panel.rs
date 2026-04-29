//! plan_ref:
//!   - 14_tech_stack#graph-visualization
//!   - 07_diff_logic#source-control-runtime
//!
//! Minimal read-only graph projection summary. Renderer work remains future.

use crate::api::fetch_graph_projection;
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::graph::GraphProjection;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Debug, PartialEq)]
enum GraphProjectionFetchState {
    Idle,
    Loading,
    Loaded(GraphProjection),
    Failed,
    Blocked,
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
        if core.active_branch.get().is_some() || core.read_block.get().is_some() {
            fetch_state.set(GraphProjectionFetchState::Blocked);
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
                    Err(_) => GraphProjectionFetchState::Failed,
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
        GraphProjectionFetchState::Failed => graph_message(
            "failed",
            t::source_control::graph_projection_unavailable(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Blocked => graph_message(
            "blocked",
            t::source_control::graph_projection_local_only(locale),
            t::source_control::graph_readonly_note(locale),
        ),
        GraphProjectionFetchState::Loaded(projection) => {
            graph_projection_summary(locale, projection)
        }
    }
}

fn graph_message(state: &'static str, message: &'static str, note: &'static str) -> AnyView {
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
    let state = if nodes == 0 { "empty" } else { "loaded" };
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
mod tests {
    use super::{GraphProjectionFetchState, graph_panel_body};
    use crate::i18n::Locale;
    use deve_core::graph::GraphProjection;

    #[test]
    fn graph_panel_copy_handles_all_fetch_states() {
        for state in [
            GraphProjectionFetchState::Idle,
            GraphProjectionFetchState::Loading,
            GraphProjectionFetchState::Failed,
            GraphProjectionFetchState::Blocked,
        ] {
            let _ = graph_panel_body(Locale::En, &state);
        }
    }

    #[test]
    fn graph_panel_loaded_summary_accepts_empty_projection() {
        let projection = GraphProjection {
            nodes: vec![],
            edges: vec![],
            unresolved_links: vec![],
        };
        let _ = graph_panel_body(Locale::Zh, &GraphProjectionFetchState::Loaded(projection));
    }
}
