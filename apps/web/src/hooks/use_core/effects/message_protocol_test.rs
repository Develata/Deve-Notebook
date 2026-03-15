#[path = "message_protocol_test_support.rs"]
mod support;

use super::clear_failed_scope_switch;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::GetUntracked;
use support::protocol_signal_harness;

#[test]
fn switch_errors_clear_pending_scope_switches_only_for_matching_nonce() {
    for code in [
        ServerErrorCode::ScRepoContextInvalid,
        ServerErrorCode::ScRepoNotSelected,
        ServerErrorCode::SyncRepoUnbound,
    ] {
        let harness = protocol_signal_harness(
            Some(PendingBranchTarget::Local),
            Some(7),
            Some("wiki"),
            Some(7),
        );
        clear_failed_scope_switch(code, Some(7), harness.control());
        harness.assert_all_requests_cleared();
        assert_eq!(harness.pending_branch_switch.get_untracked(), None);
        assert_eq!(harness.pending_repo_switch.get_untracked(), None);
    }
}

#[test]
fn switch_errors_clear_orphan_repo_switch_nonce_without_pending_name() {
    let harness = protocol_signal_harness(None, None, None, Some(7));
    clear_failed_scope_switch(
        ServerErrorCode::ScRepoContextInvalid,
        Some(7),
        harness.control(),
    );
    harness.assert_all_requests_cleared();
    assert_eq!(harness.pending_repo_switch.get_untracked(), None);
    assert_eq!(harness.pending_repo_switch_nonce.get_untracked(), None);
}

#[test]
fn stale_or_missing_nonce_keeps_pending_scope_switches() {
    for switch_nonce in [None, Some(9)] {
        let harness = protocol_signal_harness(
            Some(PendingBranchTarget::Local),
            Some(7),
            Some("wiki"),
            Some(7),
        );
        clear_failed_scope_switch(
            ServerErrorCode::ScRepoContextInvalid,
            switch_nonce,
            harness.control(),
        );
        assert_eq!(
            harness.pending_branch_switch.get_untracked(),
            Some(PendingBranchTarget::Local)
        );
        assert_eq!(
            harness.pending_repo_switch.get_untracked().as_deref(),
            Some("wiki")
        );
    }
}

#[test]
fn non_switch_errors_keep_pending_scope_switches() {
    for code in [
        ServerErrorCode::AuthTokenExpired,
        ServerErrorCode::RequestFailed,
        ServerErrorCode::StoragePersistFailed,
        ServerErrorCode::StorageDbLocked,
    ] {
        let harness = protocol_signal_harness(
            Some(PendingBranchTarget::Local),
            Some(7),
            Some("wiki"),
            Some(7),
        );
        clear_failed_scope_switch(code, Some(7), harness.control());
        assert_eq!(
            harness.pending_branch_switch.get_untracked(),
            Some(PendingBranchTarget::Local)
        );
        assert_eq!(
            harness.pending_repo_switch.get_untracked().as_deref(),
            Some("wiki")
        );
        harness.assert_all_requests_pending();
    }
}
