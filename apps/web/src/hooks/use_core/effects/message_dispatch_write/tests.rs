use super::{handle_ack_message, handle_write_ready_message};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation};
use crate::runtime::document::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use crate::hooks::use_core::state::init_signals;
use deve_core::models::{DocId, Op, PeerId, RepoId};
use leptos::prelude::{Callback, GetUntracked, Set, Update, signal};

#[test]
fn write_ready_marks_writer_without_completing_handshake() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let repo_id = RepoId::new_v4();
    let repo_id_text = repo_id.to_string();

    signals.set_current_repo_id.set(Some(repo_id_text.clone()));
    signals.set_current_scope_nonce.set(7);
    signals.set_handshake_scope_nonce.set(Some(7));
    signals.set_handshake_ready.set(false);

    handle_write_ready_message(
        PeerId::new("web-light-peer"),
        repo_id,
        7,
        None,
        &ws,
        signals,
    );

    assert!(!signals.handshake_ready.get_untracked());
    assert!(ws.writer_ready_for(Some(&repo_id_text), Some(7)));
}

#[test]
fn stale_ack_clears_matching_retained_pending_without_touching_navigation() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(91);

    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_current_scope_nonce.set(8);
    signals.set_current_doc.set(Some(doc_id));
    signals.set_pending_navigation.set(Some(PendingNavigation {
        target: NavigationTarget::Doc,
        action: Callback::new(|_| {}),
    }));
    signals.set_pending_local_edits.update(|pending| {
        push_pending_edit(
            pending,
            PendingLocalEditInput {
                repo_id,
                doc_id,
                scope_nonce: 7,
                client_id: 11,
                client_op_id: 13,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "pending".into(),
                },
            },
        );
    });

    handle_ack_message(repo_id, None, Some(7), doc_id, 1, 13, signals);

    assert_eq!(
        pending_count_for_doc(&signals.pending_local_edits.get_untracked(), doc_id),
        0
    );
    assert!(signals.pending_navigation.get_untracked().is_some());
}

#[test]
fn ack_without_scope_does_not_clear_retained_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(93);

    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_current_scope_nonce.set(8);
    signals.set_current_doc.set(Some(doc_id));
    signals.set_pending_local_edits.update(|pending| {
        push_pending_edit(
            pending,
            PendingLocalEditInput {
                repo_id,
                doc_id,
                scope_nonce: 7,
                client_id: 11,
                client_op_id: 13,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "pending".into(),
                },
            },
        );
    });

    handle_ack_message(repo_id, None, None, doc_id, 1, 13, signals);

    assert_eq!(
        pending_count_for_doc(&signals.pending_local_edits.get_untracked(), doc_id),
        1
    );
}
