//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Broadcast filter poison-lock fail-closed regression coverage.

use super::BroadcastFilter;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

fn poison_filter(filter: &BroadcastFilter) {
    let scope = filter.scope.as_ref().expect("scoped filter").clone();
    let _ = std::panic::catch_unwind(move || {
        let _guard = scope.write().expect("write lock");
        panic!("poison broadcast filter");
    });
}

#[test]
fn drops_stamped_broadcasts_when_filter_lock_is_poisoned() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);
    poison_filter(&filter);

    assert!(
        filter
            .stamp_scope_nonce(ServerMessage::CommitAck {
                repo_id: Some(uuid::Uuid::nil()),
                branch: None,
                scope_nonce: None,
                commit_id: "c1".into(),
                timestamp: 1,
            })
            .is_none()
    );
}

#[test]
fn drops_scoped_protocol_errors_when_filter_lock_is_poisoned() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);
    poison_filter(&filter);

    assert!(
        filter
            .scoped_protocol_error(ServerError::new(ServerErrorCode::RequestFailed), None)
            .is_none()
    );
}

#[test]
fn drops_broadcasts_when_filter_lock_is_poisoned() {
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    let filter = BroadcastFilter::for_session(&session);
    poison_filter(&filter);

    assert!(!filter.should_forward(&ServerMessage::CommitAck {
        repo_id: Some(uuid::Uuid::nil()),
        branch: None,
        scope_nonce: None,
        commit_id: "c1".into(),
        timestamp: 1,
    }));
}
