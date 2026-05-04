use super::{
    RefreshScope, capture_refresh_scope, should_send_refresh, should_send_refresh_through_read_gate,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::write_gate::RepoWriteGateState;
use deve_core::models::PeerId;

fn gate_state(
    connection_status: ConnectionStatus,
    is_read_only: bool,
    handshake_ready: bool,
    writer_ready: bool,
) -> RepoWriteGateState<'static> {
    RepoWriteGateState {
        connection_status,
        load_state: "ready",
        is_read_only,
        handshake_ready,
        writer_ready,
        has_repo: true,
        pending_branch_switch: false,
        pending_repo_switch: false,
    }
}

#[test]
fn does_not_capture_refresh_scope_during_switch() {
    assert_eq!(
        capture_refresh_scope(
            Some("repo-a".into()),
            None,
            Some(PendingBranchTarget::Local),
            None,
            3,
        ),
        None,
    );
}

#[test]
fn does_not_capture_refresh_scope_for_remote_branch() {
    assert_eq!(
        capture_refresh_scope(
            Some("repo-a".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
            4,
        ),
        None,
    );
}

#[test]
fn rejects_refresh_after_repo_scope_changes() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: 3,
    };
    assert!(!should_send_refresh(
        &scope,
        Some("repo-b".into()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        3,
    ));
    assert!(!should_send_refresh(
        &scope,
        Some("repo-a".into()),
        Some(PeerId::new("peer-b")),
        None,
        None,
        3,
    ));
    assert!(!should_send_refresh(
        &scope,
        Some("repo-a".into()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        4,
    ));
}

#[test]
fn keeps_refresh_only_when_scope_is_unchanged() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: None,
        scope_nonce: 5,
    };
    assert!(should_send_refresh(
        &scope,
        Some("repo-a".into()),
        None,
        None,
        None,
        5,
    ));
}

#[test]
fn refresh_read_gate_blocks_native_recovery_state() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: None,
        scope_nonce: 5,
    };

    assert!(!should_send_refresh_through_read_gate(
        &scope,
        Some("repo-a".into()),
        None,
        None,
        None,
        5,
        gate_state(ConnectionStatus::NativeServiceOffline, false, true, true),
    ));
}

#[test]
fn refresh_read_gate_requires_writer_ready_for_local_writes() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: None,
        scope_nonce: 5,
    };

    assert!(!should_send_refresh_through_read_gate(
        &scope,
        Some("repo-a".into()),
        None,
        None,
        None,
        5,
        gate_state(ConnectionStatus::Connected, false, true, false),
    ));
}

#[test]
fn refresh_read_gate_allows_spectator_reads_without_writer_ready() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: None,
        scope_nonce: 5,
    };

    assert!(should_send_refresh_through_read_gate(
        &scope,
        Some("repo-a".into()),
        None,
        None,
        None,
        5,
        gate_state(ConnectionStatus::Connected, true, false, false),
    ));
}
