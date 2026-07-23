//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Additional broadcast scope-stamping regression coverage.

use super::BroadcastFilter;
use crate::server::session::WsSession;
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ConfirmedOp, RepoListEntry, RepoReadiness, ServerMessage};

#[test]
fn stamps_runtime_broadcasts_with_session_scope_nonce() {
    let mut session = WsSession::new();
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    let commit = filter.stamp_scope_nonce(ServerMessage::CommitAck {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        commit_id: "c1".into(),
        timestamp: 1,
    });
    let fs = filter.stamp_scope_nonce(ServerMessage::FsChangeDetected {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        path: "notes/a.md".into(),
        change_type: "modified".into(),
        has_conflict: false,
    });
    let merge = filter.stamp_scope_nonce(ServerMessage::MergeComplete {
        repo_id: None,
        branch: None,
        scope_nonce: None,
        merged_count: 2,
    });
    let new_op = filter.stamp_scope_nonce(ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
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
    });
    let peer_deleted = filter.stamp_scope_nonce(ServerMessage::PeerDeleted {
        peer_id: "peer-a".into(),
        scope_nonce: None,
    });
    let repo_list = filter.stamp_scope_nonce(ServerMessage::RepoList {
        request_id: None,
        branch: None,
        scope_nonce: None,
        repo_entries: vec![],
    });

    match commit {
        Some(ServerMessage::CommitAck { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected commit message: {:?}", other),
    }
    match fs {
        Some(ServerMessage::FsChangeDetected { scope_nonce, .. }) => {
            assert_eq!(scope_nonce, Some(9))
        }
        other => panic!("unexpected fs message: {:?}", other),
    }
    match merge {
        Some(ServerMessage::MergeComplete { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected merge message: {:?}", other),
    }
    match new_op {
        Some(ServerMessage::NewOp { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected new-op message: {:?}", other),
    }
    match peer_deleted {
        Some(ServerMessage::PeerDeleted { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected peer-deleted message: {:?}", other),
    }
    match repo_list {
        Some(ServerMessage::RepoList { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected repo-list message: {:?}", other),
    }
}

#[test]
fn recipient_scope_nonce_overrides_producer_nonce_for_runtime_broadcast() {
    let mut session = WsSession::new();
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    let new_op = filter.stamp_scope_nonce(ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
        branch: None,
        scope_nonce: Some(7),
        doc_id: DocId::new(),
        entry: ConfirmedOp::new(
            1,
            Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            None,
        ),
    });

    match new_op {
        Some(ServerMessage::NewOp { scope_nonce, .. }) => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected new-op message: {:?}", other),
    }
}

#[test]
fn no_scope_browser_receives_recipient_scoped_repo_list_projection() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);
    let repo_id = uuid::Uuid::new_v4();

    let message = filter
        .stamp_scope_nonce(ServerMessage::RepoList {
            request_id: None,
            branch: None,
            scope_nonce: None,
            repo_entries: vec![RepoListEntry {
                repo_id,
                display_alias: "created".into(),
                alias_revision: 1,
                readiness: RepoReadiness::Mounted,
            }],
        })
        .expect("scope filter should remain available");

    assert!(filter.should_forward(&message));
    match message {
        ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repo_entries,
        } => {
            assert_eq!(request_id, None);
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(9));
            assert_eq!(repo_entries.len(), 1);
            assert_eq!(repo_entries[0].repo_id, repo_id);
        }
        other => panic!("unexpected repo-list message: {other:?}"),
    }
}

#[test]
fn rejects_runtime_broadcasts_with_stale_scope_nonce() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: Some(7),
        commit_id: "c1".into(),
        timestamp: 1,
    }));
    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
        branch: None,
        scope_nonce: Some(7),
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
fn rejects_repo_scoped_runtime_broadcasts_without_scope_nonce() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: None,
        commit_id: "c1".into(),
        timestamp: 1,
    }));
    assert!(!filter.should_forward(&ServerMessage::FsChangeDetected {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: None,
        path: "notes/a.md".into(),
        change_type: "modified".into(),
        has_conflict: false,
    }));
    assert!(!filter.should_forward(&ServerMessage::MergeComplete {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: None,
        merged_count: 2,
    }));
    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
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
fn rejects_peer_deleted_without_scope_nonce() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::PeerDeleted {
        peer_id: "peer-a".into(),
        scope_nonce: None,
    }));
}
