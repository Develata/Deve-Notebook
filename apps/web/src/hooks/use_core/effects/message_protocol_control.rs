//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!

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
    signals.set_search_request_id.set(None);
    signals.set_search_results.set(Vec::new());
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
        ServerErrorCode::AuthTokenExpired
            | ServerErrorCode::AuthTokenMissing
            | ServerErrorCode::AuthInvalidPassword
            | ServerErrorCode::AuthRateLimited
            | ServerErrorCode::AuthCsrfMismatch
    )
}

pub(crate) fn should_recover_scope_pref_after_failed_repo_switch(
    code: ServerErrorCode,
    switch_nonce: Option<u64>,
    pending_repo_switch_nonce: Option<u64>,
) -> bool {
    matches!(
        code,
        ServerErrorCode::ScRepoContextInvalid
            | ServerErrorCode::ScStaleScope
            | ServerErrorCode::SyncRepoUnbound
            | ServerErrorCode::StorageNotFound
    ) && switch_nonce.is_some()
        && switch_nonce == pending_repo_switch_nonce
}

#[cfg(test)]
mod tests {
    use super::should_recover_scope_pref_after_failed_repo_switch;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn scope_pref_recovery_requires_matching_repo_switch_nonce() {
        assert!(should_recover_scope_pref_after_failed_repo_switch(
            ServerErrorCode::ScRepoContextInvalid,
            Some(7),
            Some(7),
        ));
        assert!(should_recover_scope_pref_after_failed_repo_switch(
            ServerErrorCode::StorageNotFound,
            Some(9),
            Some(9),
        ));
        assert!(!should_recover_scope_pref_after_failed_repo_switch(
            ServerErrorCode::StoragePersistFailed,
            Some(7),
            Some(7),
        ));
        assert!(!should_recover_scope_pref_after_failed_repo_switch(
            ServerErrorCode::ScRepoContextInvalid,
            None,
            Some(7),
        ));
        assert!(!should_recover_scope_pref_after_failed_repo_switch(
            ServerErrorCode::ScRepoContextInvalid,
            Some(7),
            Some(9),
        ));
    }
}
