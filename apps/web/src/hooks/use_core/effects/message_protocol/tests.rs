mod support;

use super::clear_failed_scope_switch;
use super::record_search_notice;
use super::record_source_control_notice;
use crate::hooks::use_core::PendingBranchTarget;
use crate::i18n::Locale;
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::{GetUntracked, Set};
use support::protocol_signal_harness;

#[test]
fn switch_errors_clear_pending_scope_switches_only_for_matching_nonce() {
    for code in [
        ServerErrorCode::ScRepoContextInvalid,
        ServerErrorCode::ScRepoNotSelected,
        ServerErrorCode::SyncRepoUnbound,
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
fn matching_switch_nonce_always_clears_pending_scope_switches() {
    for code in [
        ServerErrorCode::AuthTokenExpired,
        ServerErrorCode::RequestFailed,
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
fn source_control_errors_are_recorded_as_panel_notice() {
    let harness = protocol_signal_harness(None, None, None, None);
    harness.control().set_search_request_id.set(None);
    let stored = record_source_control_notice(
        &ServerError::with_detail(ServerErrorCode::ScNothingToCommit, "no staged changes"),
        harness.control(),
    );
    assert!(stored);
    let notice = harness.source_control_notice.get_untracked().unwrap();
    assert_eq!(notice.code, ServerErrorCode::ScNothingToCommit);
    assert_eq!(notice.detail, None);
    harness.assert_source_control_requests_cleared();
}

#[test]
fn non_source_control_errors_do_not_record_panel_notice() {
    let harness = protocol_signal_harness(None, None, None, None);
    harness.control().set_search_request_id.set(None);
    let stored = record_source_control_notice(
        &ServerError::new(ServerErrorCode::RequestFailed),
        harness.control(),
    );
    assert!(stored);
    let notice = harness.source_control_notice.get_untracked().unwrap();
    assert_eq!(notice.code, ServerErrorCode::RequestFailed);
    harness.assert_source_control_requests_cleared();
}

#[test]
fn request_failed_without_sc_request_does_not_record_panel_notice() {
    let harness = protocol_signal_harness(None, None, None, None);
    harness.control().set_changes_request_id.set(None);
    harness.control().set_commit_history_request_id.set(None);
    harness.control().set_doc_diff_request_id.set(None);
    harness.control().set_commit_diff_request_id.set(None);
    let stored = record_source_control_notice(
        &ServerError::new(ServerErrorCode::RequestFailed),
        harness.control(),
    );
    assert!(!stored);
    assert_eq!(harness.source_control_notice.get_untracked(), None);
}

#[test]
fn search_errors_clear_search_request_and_show_banner() {
    let harness = protocol_signal_harness(None, None, None, None);
    let stored = record_search_notice(
        Locale::En,
        &ServerError::with_detail(ServerErrorCode::RequestFailed, "Search feature disabled"),
        harness.control(),
    );
    assert!(stored);
    harness.assert_search_request_cleared();
    assert_eq!(
        harness.sync_banner.get_untracked().as_deref(),
        Some("Search unavailable: Request failed")
    );
}

#[test]
fn search_errors_use_localized_fallback_banner() {
    let harness = protocol_signal_harness(None, None, None, None);
    let stored = record_search_notice(
        Locale::Zh,
        &ServerError::new(ServerErrorCode::RequestFailed),
        harness.control(),
    );
    assert!(stored);
    harness.assert_search_request_cleared();
    assert_eq!(
        harness.sync_banner.get_untracked().as_deref(),
        Some("搜索不可用: 请求失败")
    );
}

#[test]
fn non_search_errors_do_not_clear_search_state_without_pending_search() {
    let harness = protocol_signal_harness(None, None, None, None);
    harness.control().set_search_request_id.set(None);
    let stored = record_search_notice(
        Locale::En,
        &ServerError::with_detail(ServerErrorCode::RequestFailed, "other failure"),
        harness.control(),
    );
    assert!(!stored);
    assert!(!harness.search_results.get_untracked().is_empty());
    assert_eq!(harness.sync_banner.get_untracked(), None);
}
