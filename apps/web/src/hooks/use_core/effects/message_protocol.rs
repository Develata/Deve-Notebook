//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!

use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::{Locale, t};
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal};

#[path = "message_protocol_control.rs"]
mod control;
use self::control::is_auth_error;
pub(crate) use self::control::should_recover_scope_pref_after_failed_repo_switch;

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
    pub search_request_id: ReadSignal<Option<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
    pub set_search_results: WriteSignal<Vec<(String, String, f32)>>,
    pub changes_request_id: ReadSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub doc_diff_request_id: ReadSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub set_source_control_notice: WriteSignal<Option<SourceControlNotice>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
}

fn record_search_notice(error: &ServerError, signals: ProtocolControlSignals) -> bool {
    if signals.search_request_id.get_untracked().is_none() {
        return false;
    }
    signals.set_search_request_id.set(None);
    signals.set_search_results.set(Vec::new());
    let detail = error
        .detail
        .clone()
        .unwrap_or_else(|| "Search failed".to_string());
    signals
        .set_sync_banner
        .set(Some(format!("Search unavailable: {detail}")));
    true
}

fn record_source_control_notice(error: &ServerError, signals: ProtocolControlSignals) -> bool {
    if error.code == ServerErrorCode::RequestFailed && has_pending_source_control_request(signals) {
        clear_source_control_requests(signals);
        signals
            .set_source_control_notice
            .set(Some(SourceControlNotice {
                code: error.code,
                detail: error.detail.clone(),
            }));
        return true;
    }
    if let Some(notice) = SourceControlNotice::from_server_error(error) {
        clear_source_control_requests(signals);
        signals.set_source_control_notice.set(Some(notice));
        return true;
    }
    false
}

fn has_pending_source_control_request(signals: ProtocolControlSignals) -> bool {
    signals.changes_request_id.get_untracked().is_some()
        || signals.commit_history_request_id.get_untracked().is_some()
        || signals.doc_diff_request_id.get_untracked().is_some()
        || signals.commit_diff_request_id.get_untracked().is_some()
}

fn clear_source_control_requests(signals: ProtocolControlSignals) {
    signals.set_changes_request_id.set(None);
    signals.set_commit_history_request_id.set(None);
    signals.set_doc_diff_request_id.set(None);
    signals.set_commit_diff_request_id.set(None);
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
    let handled_in_search = record_search_notice(error, signals);
    let handled_in_source_control = record_source_control_notice(error, signals);
    match (
        handled_in_search,
        handled_in_source_control,
        error.detail.as_deref(),
    ) {
        (true, _, Some(detail)) => leptos::logging::log!("Search notice {}: {}", message, detail),
        (true, _, None) => leptos::logging::log!("Search notice {}", message),
        (_, true, Some(detail)) => {
            leptos::logging::log!("Source Control notice {}: {}", message, detail)
        }
        (_, true, None) => leptos::logging::log!("Source Control notice {}", message),
        (false, false, Some(detail)) => leptos::logging::warn!("协议错误 {}: {}", message, detail),
        (false, false, None) => leptos::logging::warn!("协议错误 {}", message),
    }
    if !handled_in_search
        && !handled_in_source_control
        && let Some(window) = web_sys::window()
    {
        let _ = window.alert_with_message(message);
    }
}

pub(super) fn clear_failed_scope_switch(
    code: deve_core::protocol::ServerErrorCode,
    switch_nonce: Option<u64>,
    signals: ProtocolControlSignals,
) {
    control::clear_failed_scope_switch(code, switch_nonce, signals);
}

#[cfg(test)]
#[path = "message_protocol_test.rs"]
mod tests;
