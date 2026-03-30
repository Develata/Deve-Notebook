use super::super::state::CoreSignals;
use super::message_protocol::ProtocolControlSignals;
use super::message_protocol::handle_protocol_error;
use super::message_repo_scope::{accepts_edit_rejected_message, accepts_protocol_error_message};
use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ServerError;

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
        set_changes_request_id: signals.set_changes_request_id,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
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
    handle_protocol_error(
        ws,
        locale,
        &error,
        switch_nonce,
        protocol_control_signals(signals),
    );
}
