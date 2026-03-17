use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::i18n::{Locale, t};
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal};

#[derive(Clone, Copy)]
pub struct ProtocolControlSignals {
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_branch_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    pub pending_repo_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_repo_switch: WriteSignal<Option<String>>,
    pub set_pending_repo_switch_nonce: WriteSignal<Option<u64>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub set_repo_list_request_id: WriteSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
    pub set_sync_mode_request_id: WriteSignal<Option<String>>,
    pub set_pending_ops_request_id: WriteSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
}

pub fn handle_protocol_error(
    ws: &WsService,
    locale: Locale,
    error: &ServerError,
    switch_nonce: Option<u64>,
    signals: ProtocolControlSignals,
) {
    clear_failed_scope_switch(error.code, switch_nonce, signals);
    if is_auth_error(error.code) {
        ws.mark_unauthorized();
    }
    let message = t::server_error::message(locale, error.code);
    match error.detail.as_deref() {
        Some(detail) => leptos::logging::warn!("协议错误 {}: {}", message, detail),
        None => leptos::logging::warn!("协议错误 {}", message),
    }
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(message);
    }
}

fn clear_failed_scope_switch(
    _code: ServerErrorCode,
    switch_nonce: Option<u64>,
    signals: ProtocolControlSignals,
) {
    if switch_nonce.is_none() {
        return;
    }
    let switch_nonce = switch_nonce.expect("checked above");
    let clear_branch = signals.pending_branch_switch.get_untracked().is_some()
        && signals.pending_branch_switch_nonce.get_untracked() == Some(switch_nonce);
    let clear_repo = signals.pending_repo_switch_nonce.get_untracked() == Some(switch_nonce);
    if !clear_branch && !clear_repo {
        return;
    }
    signals.set_shadow_list_request_id.set(None);
    signals.set_repo_list_request_id.set(None);
    signals.set_doc_list_request_id.set(None);
    signals.set_tree_request_id.set(None);
    signals.set_sync_mode_request_id.set(None);
    signals.set_pending_ops_request_id.set(None);
    signals.set_changes_request_id.set(None);
    signals.set_commit_history_request_id.set(None);
    signals.set_doc_diff_request_id.set(None);
    signals.set_commit_diff_request_id.set(None);
    if clear_branch {
        signals.set_pending_branch_switch.set(None);
        signals.set_pending_branch_switch_nonce.set(None);
    }
    if clear_repo {
        signals.set_pending_repo_switch.set(None);
        signals.set_pending_repo_switch_nonce.set(None);
    }
}

fn is_auth_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing
    )
}

#[cfg(test)]
#[path = "message_protocol_test.rs"]
mod tests;
