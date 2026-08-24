use super::handle_doc_list;
use super::selection::{query_doc_path_from_search, reconcile_doc_selection_for_query};
use crate::hooks::use_core::state_init::init_signals;
use crate::runtime::document::create::PendingDocumentCreate;
use deve_core::models::{DocId, RepoId};
use deve_core::protocol::{
    DocumentCreateProjectionOutcome, DocumentCreateResponse, DocumentCreateResponseContext,
};
use leptos::prelude::*;

#[test]
fn doc_list_clears_stale_current_doc() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
    let stale = DocId::new();
    let fresh = DocId::new();
    signals.set_current_doc.set(Some(stale));

    handle_doc_list(
        None,
        None,
        None,
        Some(0),
        vec![(fresh, "notes/fresh.md".into())],
        signals,
    );

    assert_eq!(signals.current_doc.get_untracked(), None);
}

#[test]
fn doc_list_preserves_current_doc_when_it_still_exists() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
    let selected = DocId::new();
    signals.set_current_doc.set(Some(selected));

    handle_doc_list(
        None,
        None,
        None,
        Some(0),
        vec![(selected, "notes/selected.md".into())],
        signals,
    );

    assert_eq!(signals.current_doc.get_untracked(), Some(selected));
}

#[test]
fn doc_list_selects_pending_created_doc_when_no_doc_is_open() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
    let pending = confirmed_pending_create("Untitled.md", true);
    let created = DocId(pending.proposed_node_id().0);
    signals.set_pending_document_create.set(Some(pending));

    handle_doc_list(
        None,
        None,
        None,
        Some(0),
        vec![(created, "Untitled.md".into())],
        signals,
    );

    assert_eq!(signals.current_doc.get_untracked(), Some(created));
    assert_eq!(signals.pending_document_create.get_untracked(), None);
}

#[test]
fn doc_list_selects_query_doc_when_no_doc_is_open() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
    let first = DocId::new();
    let requested = DocId::new();

    reconcile_doc_selection_for_query(
        &[
            (first, "notes/first.md".into()),
            (requested, "notes/requested.md".into()),
        ],
        signals,
        Some("notes/requested.md"),
    );

    assert_eq!(signals.current_doc.get_untracked(), Some(requested));
}

#[test]
fn pending_created_doc_selection_wins_over_query_doc() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
    let pending = confirmed_pending_create("Untitled.md", true);
    let created = DocId(pending.proposed_node_id().0);
    let requested = DocId::new();
    signals.set_pending_document_create.set(Some(pending));

    reconcile_doc_selection_for_query(
        &[
            (created, "Untitled.md".into()),
            (requested, "notes/requested.md".into()),
        ],
        signals,
        Some("notes/requested.md"),
    );

    assert_eq!(signals.current_doc.get_untracked(), Some(created));
    assert_eq!(signals.pending_document_create.get_untracked(), None);
}

fn confirmed_pending_create(path: &str, select: bool) -> PendingDocumentCreate {
    let mut pending = PendingDocumentCreate::new(RepoId::new_v4(), 7, path.into(), select);
    let request = pending.request();
    let doc_id = DocId(request.proposed_node_id.0);
    let response = DocumentCreateResponse::Created {
        context: DocumentCreateResponseContext::from(&request),
        node_id: request.proposed_node_id,
        doc_id: Some(doc_id),
        path: request.path,
        projection_outcome: DocumentCreateProjectionOutcome::Written,
    };
    pending.accept_response(&response);
    pending
}

#[test]
fn query_doc_path_parser_decodes_doc_param() {
    assert_eq!(
        query_doc_path_from_search("?sc_full=1&doc=notes%2Fhello%20world.md"),
        Some("notes/hello world.md".to_string())
    );
}

#[test]
fn query_doc_path_parser_rejects_empty_or_malformed_doc_param() {
    assert_eq!(query_doc_path_from_search("?doc="), None);
    assert_eq!(query_doc_path_from_search("?doc=notes%2"), None);
    assert_eq!(query_doc_path_from_search("?other=notes.md"), None);
}
