use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::{Locale, t};
use deve_core::protocol::ServerError;
use leptos::prelude::{ReadSignal, Set, WriteSignal};

#[path = "message_protocol_control.rs"]
mod control;
use self::control::is_auth_error;

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
    pub set_source_control_notice: WriteSignal<Option<SourceControlNotice>>,
}

fn record_source_control_notice(
    error: &ServerError,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
) -> bool {
    if let Some(notice) = SourceControlNotice::from_server_error(error) {
        set_notice.set(Some(notice));
        return true;
    }
    false
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
    let handled_in_source_control =
        record_source_control_notice(error, signals.set_source_control_notice);
    match error.detail.as_deref() {
        Some(detail) => leptos::logging::warn!("协议错误 {}: {}", message, detail),
        None => leptos::logging::warn!("协议错误 {}", message),
    }
    if !handled_in_source_control {
        if let Some(window) = web_sys::window() {
            let _ = window.alert_with_message(message);
        }
    }
}

fn clear_failed_scope_switch(
    code: deve_core::protocol::ServerErrorCode,
    switch_nonce: Option<u64>,
    signals: ProtocolControlSignals,
) {
    control::clear_failed_scope_switch(code, switch_nonce, signals);
}

#[cfg(test)]
#[path = "message_protocol_test.rs"]
mod tests;
