use super::BroadcastFilter;
use crate::server::session::WsSession;
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::{ConfirmedOp, ServerMessage};

#[test]
fn rejects_new_op_from_other_branch() {
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);

    assert!(!filter.should_forward(&ServerMessage::NewOp {
        repo_id: uuid::Uuid::nil(),
        branch: Some(PeerId::new("peer-b")),
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

    match commit {
        ServerMessage::CommitAck { scope_nonce, .. } => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected commit message: {:?}", other),
    }
    match fs {
        ServerMessage::FsChangeDetected { scope_nonce, .. } => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected fs message: {:?}", other),
    }
    match merge {
        ServerMessage::MergeComplete { scope_nonce, .. } => assert_eq!(scope_nonce, Some(9)),
        other => panic!("unexpected merge message: {:?}", other),
    }
}

#[test]
fn drops_broadcasts_when_filter_lock_is_poisoned() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);
    let scope = filter.scope.as_ref().expect("scoped filter").clone();
    let _ = std::panic::catch_unwind(move || {
        let _guard = scope.write().expect("write lock");
        panic!("poison broadcast filter");
    });

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: None,
        commit_id: "c1".into(),
        timestamp: 1,
    }));
}
