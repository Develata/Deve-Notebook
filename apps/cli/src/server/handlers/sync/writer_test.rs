//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::*;
use deve_core::ledger::database::DatabaseHandle;
use std::sync::Arc;

#[test]
fn rejects_unbound_repo() {
    let mut session = WsSession::new();
    let error = validate(&mut session, uuid::Uuid::nil(), &PeerId::new("browser"), 1).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
}

#[test]
fn rejects_readonly_writer_registration() {
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::new_v4();
    let peer_id = PeerId::new("browser");
    set_db(
        &mut session,
        true,
        Some(PeerId::new("remote")),
        None,
        "repo",
    );
    session.set_authenticated(peer_id.clone());
    let error = validate(&mut session, repo_id, &peer_id, 1).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScRemoteBranchReadonly);
}

#[test]
fn rejects_mismatched_peer() {
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::new_v4();
    session.set_authenticated(PeerId::new("browser-a"));
    session.bind_repo(repo_id);
    let error = validate(&mut session, repo_id, &PeerId::new("browser-b"), 1).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_sync_cleared(&session);
}

#[test]
fn accepts_bound_matching_peer() {
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::new_v4();
    let peer_id = PeerId::new("browser-a");
    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    assert!(validate(&mut session, repo_id, &peer_id, 1).is_ok());
}

#[test]
fn rejects_browser_writer_with_stale_scope_nonce() {
    let (mut session, repo_id, peer_id) = browser_session(new_repo_id());
    let error = validate(&mut session, repo_id, &peer_id, 8).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

#[test]
fn rejects_browser_writer_for_non_active_repo() {
    let repo_id = new_repo_id();
    let (mut session, _, peer_id) = browser_session(repo_id);
    session.switch_repo("notes".into(), Some(new_repo_id()));
    set_db(&mut session, false, None, Some(repo_id), "notes");
    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

#[test]
fn rejects_browser_writer_with_stale_remote_readonly_binding() {
    let repo_id = new_repo_id();
    let (mut session, _, peer_id) = browser_session(repo_id);
    set_db(
        &mut session,
        true,
        Some(PeerId::new("remote")),
        Some(new_repo_id()),
        "shadow",
    );
    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

fn browser_session(repo_id: RepoId) -> (WsSession, RepoId, PeerId) {
    let mut session = WsSession::new();
    let peer_id = PeerId::new("browser-a");
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    session.set_sync_scope_nonce(9);
    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    (session, repo_id, peer_id)
}

fn set_db(
    session: &mut WsSession,
    readonly: bool,
    branch: Option<PeerId>,
    repo_id: Option<RepoId>,
    repo_name: &str,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(redb::Database::create(dir.path().join("remote.redb")).expect("db"));
    session.set_active_db(DatabaseHandle {
        db,
        readonly,
        branch,
        repo_id,
        repo_name: repo_name.into(),
    });
}

fn assert_sync_cleared(session: &WsSession) {
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(session.writer_identity.is_none());
}

fn new_repo_id() -> RepoId {
    uuid::Uuid::new_v4()
}
