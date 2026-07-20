//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! WebSocket broadcast filter regression coverage.

use super::BroadcastFilter;
use crate::server::session::WsSession;
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::{
    ConfirmedOp, ProjectionRecoveryCause, ProjectionRecoveryPlan, ProjectionRecoveryRequired,
    ServerMessage,
};

fn recovery(
    repo_id: uuid::Uuid,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
) -> ServerMessage {
    ServerMessage::ProjectionRecoveryRequired(ProjectionRecoveryRequired {
        repo_id,
        branch,
        scope_nonce,
        cause: ProjectionRecoveryCause::DocumentMutation,
        plan: ProjectionRecoveryPlan::external_apply(vec![DocId::new()]),
    })
}

#[test]
fn rejects_new_op_from_other_branch() {
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
        branch: Some(PeerId::new("peer-b")),
        scope_nonce: None,
        doc_id: DocId::new(),
        entry: ConfirmedOp::new(
            1,
            Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            None,
        ),
    }));
}

#[test]
fn rejects_unscoped_runtime_broadcasts_for_bound_repo_sessions() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        commit_id: "c1".into(),
        timestamp: 1,
    }));
    assert!(!filter.should_forward(&ServerMessage::FsChangeDetected {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        path: "notes/a.md".into(),
        change_type: "modified".into(),
        has_conflict: false,
    }));
    assert!(!filter.should_forward(&ServerMessage::MergeComplete {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        merged_count: 2,
    }));
}

#[test]
fn rejects_new_op_for_wrong_local_repo() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::new_v4()));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::new_v4(),
        branch: None,
        scope_nonce: None,
        doc_id: DocId::new(),
        entry: ConfirmedOp::new(
            1,
            Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            None,
        ),
    }));
}

#[test]
fn rejects_new_op_for_unbound_session() {
    let session = WsSession::new();
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::new_v4(),
        branch: None,
        scope_nonce: None,
        doc_id: DocId::new(),
        entry: ConfirmedOp::new(
            1,
            Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            None,
        ),
    }));
}

#[test]
fn rejects_repo_scoped_runtime_broadcasts_for_unbound_sessions() {
    let session = WsSession::new();
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: Some(7),
        commit_id: "c1".into(),
        timestamp: 1,
    }));
    assert!(!filter.should_forward(&ServerMessage::FsChangeDetected {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: Some(7),
        path: "notes/a.md".into(),
        change_type: "modified".into(),
        has_conflict: false,
    }));
    assert!(!filter.should_forward(&ServerMessage::MergeComplete {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: Some(7),
        merged_count: 2,
    }));
}

#[test]
fn forwards_peer_deleted_to_unbound_browser_sessions() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));
    let filter = BroadcastFilter::for_session(&session);

    assert!(filter.should_forward(&ServerMessage::PeerDeleted {
        peer_id: "peer-a".into(),
        scope_nonce: Some(7),
    }));
}

#[test]
fn rejects_peer_deleted_for_non_browser_unbound_sessions() {
    let session = WsSession::new();
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::PeerDeleted {
        peer_id: "peer-a".into(),
        scope_nonce: Some(7),
    }));
}

#[test]
fn rejects_host_local_repo_projections_for_full_peer() {
    let session = WsSession::new();
    let filter = BroadcastFilter::for_session(&session);
    let repo_id = uuid::Uuid::new_v4();

    assert!(!filter.should_forward(&ServerMessage::RepoList {
        request_id: None,
        branch: None,
        scope_nonce: None,
        repo_entries: vec![deve_core::protocol::RepoListEntry {
            repo_id,
            display_alias: "HOST_SECRET_ALIAS".into(),
            alias_revision: 1,
            readiness: deve_core::protocol::RepoReadiness::Mounted,
        }],
    }));
    assert!(!filter.should_forward(&ServerMessage::RepoSwitched {
        branch: None,
        repo_id,
        display_alias: "HOST_SECRET_ALIAS".into(),
        switch_nonce: Some(1),
        scope_nonce: deve_core::protocol::ScopeNonce::new(1),
    }));
}

#[test]
fn projection_recovery_requires_exact_repo_branch_and_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    assert!(filter.should_forward(&recovery(repo_id, None, Some(9))));
    assert!(!filter.should_forward(&recovery(uuid::Uuid::new_v4(), None, Some(9))));
    assert!(!filter.should_forward(&recovery(repo_id, None, Some(8))));
    assert!(!filter.should_forward(&recovery(repo_id, None, None)));

    session.switch_branch(Some("peer-a".into()));
    let remote_filter = BroadcastFilter::for_session(&session);
    assert!(!remote_filter.should_forward(&recovery(
        repo_id,
        Some(PeerId::new("peer-b")),
        Some(9),
    )));
}

mod extra;
