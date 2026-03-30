use deve_core::protocol::ServerErrorCode;
use leptos::prelude::{GetUntracked, Set};

use super::ProtocolControlSignals;

pub(super) fn clear_failed_scope_switch(
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

pub(super) fn is_auth_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing
    )
}
