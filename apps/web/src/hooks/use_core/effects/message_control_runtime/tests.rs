use super::super::message_control_runtime_repo::{next_request_id, should_request_repo_sync_state};
use crate::api::ConnectionStatus;
use crate::runtime::document::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use crate::hooks::use_core::state::init_signals;
use deve_core::models::{DocId, Op, RepoId};
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Update, signal};

#[test]
fn repo_sync_state_requests_only_run_on_local_branch() {
    assert!(should_request_repo_sync_state(None));
    assert!(!should_request_repo_sync_state(Some(
        deve_core::models::PeerId::new("peer-a")
    )));
}

#[test]
fn request_ids_are_non_empty() {
    let request_id = next_request_id();
    assert!(!request_id.is_empty());
    assert!(uuid::Uuid::parse_str(&request_id).is_ok());
}

#[test]
fn list_repos_request_keeps_shared_request_id_shape() {
    let request_id = next_request_id();
    let msg = ClientMessage::ListRepos {
        request_id: request_id.clone(),
        scope_nonce: Some(7),
    };
    assert!(matches!(
        msg,
        ClientMessage::ListRepos {
            request_id: actual,
            scope_nonce: Some(7),
        } if actual == request_id
    ));
}

#[test]
fn repo_scoped_runtime_reset_preserves_pending_overlay_rows() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let signals = init_signals(signal(ConnectionStatus::Connected).0);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(91);
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

    super::clear_repo_scoped_runtime(signals);

    assert_eq!(
        pending_count_for_doc(&signals.pending_local_edits.get_untracked(), doc_id),
        1
    );
}
