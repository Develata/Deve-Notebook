use super::super::state::CoreSignals;
use super::super::scope_prefs::clear_scope_pref;
use super::message_control_runtime_repo::{clear_repo_scoped_runtime, request_repo_list};
use super::message_protocol::ProtocolControlSignals;
use super::message_protocol::{
    clear_failed_scope_switch, handle_protocol_error,
    should_recover_scope_pref_after_failed_repo_switch,
};
use super::message_repo_scope::{accepts_edit_rejected_message, accepts_protocol_error_message};
use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ServerError;
use leptos::prelude::{GetUntracked, Set};

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
        changes_request_id: signals.changes_request_id,
        set_changes_request_id: signals.set_changes_request_id,
        commit_history_request_id: signals.commit_history_request_id,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        doc_diff_request_id: signals.doc_diff_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
        commit_diff_request_id: signals.commit_diff_request_id,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
        set_source_control_notice: signals.set_source_control_notice,
    }
}

pub fn handle_edit_rejected_message(
    scope_nonce: Option<u64>,
    error: ServerError,
    ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) {
    if !accepts_edit_rejected_message(scope_nonce, signals) {
        return;
    }
    handle_protocol_error(ws, locale, &error, None, protocol_control_signals(signals));
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
