use super::{
    GraphProjectionFetchState, graph_blocked_note, graph_loaded_state_attr, graph_panel_body,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::Locale;
use deve_core::graph::{GraphNode, GraphProjection};
use deve_core::models::DocId;

#[test]
fn graph_panel_copy_handles_all_fetch_states() {
    for state in [
        GraphProjectionFetchState::Idle,
        GraphProjectionFetchState::Loading,
        GraphProjectionFetchState::Error,
        GraphProjectionFetchState::Blocked(RepoWriteBlock::Offline),
        GraphProjectionFetchState::LocalOnly,
        GraphProjectionFetchState::Degraded,
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

#[test]
fn graph_panel_state_attrs_are_acceptance_stable() {
    let empty = GraphProjection {
        nodes: vec![],
        edges: vec![],
        unresolved_links: vec![],
    };
    let non_empty = GraphProjection {
        nodes: vec![GraphNode {
            doc_id: DocId::from_u128(1),
            path: "notes/a.md".into(),
            title: "a".into(),
        }],
        edges: vec![],
        unresolved_links: vec![],
    };

    assert_eq!(graph_loaded_state_attr(&empty), "empty");
    assert_eq!(graph_loaded_state_attr(&non_empty), "loaded");
}

#[test]
fn graph_panel_blocked_note_includes_gate_reason() {
    let note = graph_blocked_note(Locale::En, RepoWriteBlock::Offline);
    assert!(note.contains("Offline"));
    assert!(note.contains(": "));
}
