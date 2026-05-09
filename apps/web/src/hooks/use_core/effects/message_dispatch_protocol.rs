//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use super::super::scope_prefs::clear_scope_pref;
use super::super::state::CoreSignals;
use super::message_control_runtime_repo::{clear_repo_scoped_runtime, request_repo_list};
use super::message_protocol::ProtocolControlSignals;
use super::message_protocol::{
    clear_failed_scope_switch, handle_protocol_error,
    should_recover_scope_pref_after_failed_repo_switch,
};
use super::message_repo_scope::{accepts_edit_rejected_message, accepts_protocol_error_message};
use crate::api::WsService;
use crate::hooks::use_core::pending;
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::Locale;
use deve_core::models::DocId;
use deve_core::protocol::ServerError;
use leptos::prelude::{GetUntracked, Set, Update};

pub fn protocol_control_signals(signals: CoreSignals) -> ProtocolControlSignals {
    ProtocolControlSignals {
        pending_branch_switch: signals.pending_branch_switch,
        pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
        set_pending_branch_switch: signals.set_pending_branch_switch,
        set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
        pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
        set_pending_repo_switch: signals.set_pending_repo_switch,
        set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
        set_shadow_list_request_id: signals.set_shadow_list_request_id,
        set_repo_list_request_id: signals.set_repo_list_request_id,
        set_doc_list_request_id: signals.set_doc_list_request_id,
        set_tree_request_id: signals.set_tree_request_id,
        set_sync_mode_request_id: signals.set_sync_mode_request_id,
        set_pending_ops_request_id: signals.set_pending_ops_request_id,
        search_request_id: signals.search_request_id,
        set_search_request_id: signals.set_search_request_id,
        set_search_results: signals.set_search_results,
        changes_request_id: signals.changes_request_id,
        set_changes_request_id: signals.set_changes_request_id,
        commit_history_request_id: signals.commit_history_request_id,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        doc_diff_request_id: signals.doc_diff_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
        commit_diff_request_id: signals.commit_diff_request_id,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
        set_source_control_notice: signals.set_source_control_notice,
        set_sync_banner: signals.set_sync_banner,
    }
}

pub fn handle_edit_rejected_message(
    scope_nonce: u64,
    doc_id: DocId,
    client_op_id: u64,
    error: ServerError,
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) {
    let accepts_current_scope = accepts_edit_rejected_message(Some(scope_nonce), signals);
    let has_matching_pending = pending::has_pending_edit(
        &signals.pending_local_edits.get_untracked(),
        None,
        Some(scope_nonce),
        doc_id,
        client_op_id,
    );
    if !accepts_current_scope && !has_matching_pending {
        return;
    }
    let current_doc = accepts_current_scope
        .then(|| signals.current_doc.get_untracked())
        .flatten();
    let mut clear_navigation = false;
    signals.set_pending_local_edits.update(|pending_edits| {
        clear_navigation = pending::clear_pending_edit_and_check_current_doc_empty(
            pending_edits,
            current_doc,
            None,
            Some(scope_nonce),
            doc_id,
            client_op_id,
        );
    });
    if clear_navigation {
        signals.set_pending_navigation.set(None);
    }
    if accepts_current_scope {
        handle_protocol_error(ws, locale, &error, None, protocol_control_signals(signals));
    }
}

pub fn handle_protocol_error_message(
    error: ServerError,
    switch_nonce: Option<u64>,
    scope_nonce: Option<u64>,
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) {
    if !accepts_protocol_error_message(scope_nonce, switch_nonce, signals) {
        return;
    }
    if should_recover_scope_pref_after_failed_repo_switch(
        error.code,
        switch_nonce,
        signals.pending_repo_switch_nonce.get_untracked(),
    ) {
        clear_failed_scope_switch(error.code, switch_nonce, protocol_control_signals(signals));
        recover_from_failed_scope_restore(ws, signals);
        leptos::logging::warn!(
            "自动清理失效的 repo scope 偏好并重新请求仓库列表: code={:?}",
            error.code
        );
        return;
    }
    handle_protocol_error(
        ws,
        locale,
        &error,
        switch_nonce,
        protocol_control_signals(signals),
    );
    finish_pending_chat_on_protocol_error(&error, locale, signals);
}

fn finish_pending_chat_on_protocol_error(
    error: &ServerError,
    locale: Locale,
    signals: CoreSignals,
) -> bool {
    if !signals.is_chat_streaming.get_untracked() {
        return false;
    }
    let pending = signals.plugin_request_ids.get_untracked();
    if pending.is_empty() {
        return false;
    }

    let text = chat_protocol_error_text(error, locale);
    let mut matched = false;
    let mut matched_ids = Vec::new();
    signals.set_chat_messages.update(|messages| {
        for req_id in &pending {
            if let Some(message) = messages
                .iter_mut()
                .rev()
                .find(|msg| msg.req_id.as_deref() == Some(req_id.as_str()))
            {
                append_chat_protocol_error(message, &text);
                matched_ids.push(req_id.clone());
                matched = true;
            }
        }
    });
    if !matched_ids.is_empty() {
        signals
            .set_plugin_request_ids
            .update(|ids| ids.retain(|id| !matched_ids.iter().any(|matched_id| matched_id == id)));
    }
    if matched {
        signals.set_is_chat_streaming.set(false);
    }
    matched
}

fn chat_protocol_error_text(error: &ServerError, locale: Locale) -> String {
    error
        .detail
        .clone()
        .unwrap_or_else(|| crate::i18n::t::server_error::message(locale, error.code).to_string())
}

fn append_chat_protocol_error(message: &mut ChatMessage, detail: &str) {
    if detail.is_empty() || message.content.contains(detail) {
        return;
    }
    if !message.content.is_empty() {
        message.content.push_str("\n\n");
    }
    message.content.push_str(detail);
}

fn recover_from_failed_scope_restore(ws: &WsService, signals: CoreSignals) {
    clear_scope_pref();
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_active_branch.set(None);
    signals.set_current_repo.set(None);
    signals.set_current_repo_id.set(None);
    signals.set_current_doc.set(None);
    signals.set_docs.set(Vec::new());
    signals.set_tree_nodes.set(Vec::new());
    signals.set_repo_list.set(Vec::new());
    clear_repo_scoped_runtime(signals);
    request_repo_list(ws, signals);
}

#[cfg(test)]
mod tests {
    use super::{finish_pending_chat_on_protocol_error, handle_edit_rejected_message};
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation};
    use crate::hooks::use_core::pending::{
        PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
    };
    use crate::hooks::use_core::state::init_signals;
    use crate::hooks::use_core::types::ChatMessage;
    use crate::i18n::Locale;
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
                "Invalid bincode client message"
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
            "Invalid bincode client message"
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
            "partial\n\ntransport failed"
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
}
