use super::handle_doc_list;
use crate::hooks::use_core::state_init::init_signals;
use deve_core::models::DocId;
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
    let created = DocId::new();
    signals
        .set_pending_created_doc_path
        .set(Some("Untitled.md".to_string()));

    handle_doc_list(
        None,
        None,
        None,
        Some(0),
        vec![(created, "Untitled.md".into())],
        signals,
    );

    assert_eq!(signals.current_doc.get_untracked(), Some(created));
    assert_eq!(signals.pending_created_doc_path.get_untracked(), None);
}
