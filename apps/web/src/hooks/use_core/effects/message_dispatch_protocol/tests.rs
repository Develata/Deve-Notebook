use super::{
    finish_pending_chat_on_protocol_error, handle_edit_rejected_message,
    handle_protocol_error_message,
};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation};
use crate::hooks::use_core::state::init_signals;
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::Locale;
use crate::runtime::document::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use deve_core::models::{DocId, Op, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::*;

#[test]
fn protocol_error_finishes_pending_chat_placeholder() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_is_chat_streaming.set(true);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_chat_messages.set(vec![ChatMessage {
        role: "assistant".into(),
        content: String::new(),
        req_id: Some("req-1".into()),
        ts_ms: 0,
    }]);

    assert!(finish_pending_chat_on_protocol_error(
        &ServerError::with_detail(
            ServerErrorCode::RequestFailed,
            "Invalid binary client message"
        ),
        Locale::En,
        signals,
    ));

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.plugin_request_ids.get_untracked(),
        Vec::<String>::new()
    );
    assert_eq!(
        signals.chat_messages.get_untracked()[0].content,
        "Request failed"
    );
}

#[test]
fn protocol_error_appends_after_partial_chat_content() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_is_chat_streaming.set(true);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_chat_messages.set(vec![ChatMessage {
        role: "assistant".into(),
        content: "partial".into(),
        req_id: Some("req-1".into()),
        ts_ms: 0,
    }]);

    assert!(finish_pending_chat_on_protocol_error(
        &ServerError::with_detail(ServerErrorCode::RequestFailed, "transport failed"),
        Locale::En,
        signals,
    ));

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.chat_messages.get_untracked()[0].content,
        "partial\n\nRequest failed"
    );
}

#[test]
fn protocol_error_does_not_clear_unmatched_chat_request() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_is_chat_streaming.set(true);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_chat_messages.set(vec![ChatMessage {
        role: "assistant".into(),
        content: String::new(),
        req_id: Some("other-req".into()),
        ts_ms: 0,
    }]);

    assert!(!finish_pending_chat_on_protocol_error(
        &ServerError::with_detail(ServerErrorCode::RequestFailed, "transport failed"),
        Locale::En,
        signals,
    ));

    assert!(signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.plugin_request_ids.get_untracked(),
        vec!["req-1".to_string()]
    );
}

#[test]
fn unscoped_auth_protocol_error_marks_unauthorized() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    handle_protocol_error_message(
        ServerError::new(ServerErrorCode::AuthTokenExpired),
        None,
        None,
        &ws,
        Locale::En,
        signals,
    );

    assert_eq!(ws.status.get_untracked(), ConnectionStatus::Unauthorized);
    assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
    assert!(ws.writer_client_id.get_untracked().is_none());
}

#[test]
fn unscoped_non_auth_protocol_error_without_switch_is_ignored() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let ws = WsService::new_for_test(ConnectionStatus::Connected);

    handle_protocol_error_message(
        ServerError::new(ServerErrorCode::RequestFailed),
        None,
        None,
        &ws,
        Locale::En,
        signals,
    );

    assert_eq!(ws.status.get_untracked(), ConnectionStatus::Connected);
    assert_eq!(signals.sync_banner.get_untracked(), None);
}

#[test]
fn scoped_workspace_ingestion_protocol_error_binds_only_current_scope() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let ws = WsService::new_with_incoming_for_test(
        ConnectionStatus::Connected,
        3,
        std::collections::VecDeque::new(),
    );
    let repo_id = RepoId::new_v4();
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_current_scope_nonce.set(7);
    let error = ServerError::with_detail(
        ServerErrorCode::StorageWorkspaceIngestionUnavailable,
        "CANARY_PRIVATE_BACKEND_DETAIL",
    );

    handle_protocol_error_message(error.clone(), None, Some(6), &ws, Locale::En, signals);
    assert!(!ws.workspace_ingestion_blocked_for_untracked(Some(&repo_id.to_string()), Some(7)));

    handle_protocol_error_message(error, None, Some(7), &ws, Locale::En, signals);
    assert!(ws.workspace_ingestion_blocked_for_untracked(Some(&repo_id.to_string()), Some(7)));
}

#[test]
fn stale_edit_rejected_clears_matching_retained_pending_without_banner() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(92);

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

    handle_edit_rejected_message(
        7,
        doc_id,
        13,
        ServerError::with_detail(ServerErrorCode::SyncEditRejected, "old scope rejected"),
        &ws,
        Locale::En,
        signals,
    );

    assert_eq!(
        pending_count_for_doc(&signals.pending_local_edits.get_untracked(), doc_id),
        0
    );
    assert!(signals.pending_navigation.get_untracked().is_some());
    assert!(signals.sync_banner.get_untracked().is_none());
}
