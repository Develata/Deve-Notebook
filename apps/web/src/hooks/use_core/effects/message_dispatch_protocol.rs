//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use super::super::scope_prefs::clear_scope_pref;
use super::super::state::CoreSignals;
use super::message_control_runtime_repo::{clear_repo_scoped_runtime, request_repo_list};
use super::message_protocol::ProtocolControlSignals;
use super::message_protocol::{
    clear_failed_scope_switch, handle_protocol_error, is_session_auth_error,
    should_recover_scope_pref_after_failed_repo_switch,
};
use super::message_remove_scope;
use super::message_repo_scope::{accepts_edit_rejected_message, accepts_protocol_error_message};
use crate::api::WsService;
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::Locale;
use crate::runtime::document::{confirm, pending};
use deve_core::models::DocId;
use deve_core::protocol::ServerError;
use leptos::prelude::{GetUntracked, Set, Update};

pub fn protocol_control_signals(signals: CoreSignals) -> ProtocolControlSignals {
    ProtocolControlSignals {
        pending_branch_switch: signals.pending_branch_switch,
        set_pending_branch_switch: signals.set_pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
        set_pending_repo_switch: signals.set_pending_repo_switch,
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
        current_repo_id: signals.current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
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
    let mut rejected_waiting_edit = false;
    signals.set_pending_local_edits.update(|pending_edits| {
        let resolution = confirm::reject_pending_edit(
            pending_edits,
            current_doc,
            Some(scope_nonce),
            doc_id,
            client_op_id,
        );
        clear_navigation = resolution.clear_navigation;
        rejected_waiting_edit = resolution.confirmation.is_some_and(|c| c.is_failed());
    });
    if rejected_waiting_edit {
        leptos::logging::warn!(
            "本地编辑被服务端拒绝并撤回: doc={doc_id} client_op_id={client_op_id} code={:?}",
            error.code
        );
    }
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
    if message_remove_scope::capture_protocol_error(
        error.code,
        switch_nonce,
        scope_nonce,
        ws,
        locale,
        signals,
    ) {
        return;
    }
    if !is_session_auth_error(error.code)
        && !accepts_protocol_error_message(scope_nonce, switch_nonce, signals)
    {
        return;
    }
    if should_recover_scope_pref_after_failed_repo_switch(
        error.code,
        switch_nonce,
        signals
            .pending_repo_switch
            .get_untracked()
            .map(|pending| pending.switch_nonce),
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
    crate::i18n::t::server_error::message(locale, error.code).to_string()
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
    signals.set_repo_entries.set(Vec::new());
    clear_repo_scoped_runtime(signals);
    request_repo_list(ws, signals);
}

#[cfg(test)]
mod tests;
