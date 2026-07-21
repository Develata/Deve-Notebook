//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::*;
use deve_core::ledger::database::DatabaseHandle;
use std::sync::Arc;

#[test]
fn rejects_unbound_repo() {
    let repo_id = new_repo_id();
    let peer_id = PeerId::new("browser");
    let mut session = browser_session_without_repo_binding(repo_id, peer_id.clone());
    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
}

#[test]
fn rejects_stale_readonly_binding_in_local_scope() {
    let (mut session, repo_id, peer_id) = browser_session(new_repo_id());
    set_db(&mut session, true, None, Some(repo_id), "notes");
    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

#[test]
fn rejects_mismatched_peer() {
    let (mut session, repo_id, _) = browser_session(new_repo_id());
    let error = validate(&mut session, repo_id, &PeerId::new("browser-b"), 9).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_sync_cleared(&session);
}

#[test]
fn accepts_bound_matching_peer() {
    let (mut session, repo_id, peer_id) = browser_session(new_repo_id());
    assert!(validate(&mut session, repo_id, &peer_id, 9).is_ok());
}

#[test]
fn rejects_fullpeer_writer_registration_without_browser_session() {
    let mut session = WsSession::new();
    let repo_id = new_repo_id();
    let peer_id = PeerId::new("full-peer");
    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(9);

    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();

    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_eq!(
        error.detail.as_deref(),
        Some("writer registration requires browser session")
    );
    assert_eq!(session.authenticated_peer_id.as_ref(), Some(&peer_id));
    assert_eq!(session.bound_repo_id, Some(repo_id));
    assert_eq!(session.sync_scope_nonce(), Some(9));
    assert!(session.writer_identity.is_none());
}

#[test]
fn rejects_browser_writer_with_stale_scope_nonce() {
    let (mut session, repo_id, peer_id) = browser_session(new_repo_id());
    let error = validate(&mut session, repo_id, &peer_id, 8).unwrap_err();
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
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

#[test]
fn rejects_browser_writer_on_remote_branch_and_clears_stale_writer() {
    let repo_id = new_repo_id();
    let (mut session, _, peer_id) = browser_session(repo_id);
    session.set_writer_identity(repo_id, peer_id.clone(), 9);
    session.switch_branch(Some("remote".into()));

    let error = validate(&mut session, repo_id, &peer_id, 9).unwrap_err();

    assert_eq!(error.code, ServerErrorCode::ScRemoteBranchReadonly);
    assert_sync_cleared(&session);
}

#[test]
fn writer_identity_requires_matching_repo_and_scope_nonce() {
    let (mut session, repo_id, peer_id) = browser_session(new_repo_id());
    session.set_writer_identity(repo_id, peer_id.clone(), 9);

    assert_eq!(session.writer_peer_id_for(&repo_id, Some(9)), Some(peer_id));
    assert_eq!(session.writer_peer_id_for(&repo_id, Some(8)), None);
    assert_eq!(session.writer_peer_id_for(&repo_id, None), None);
    assert_eq!(
        session.writer_peer_id_for(&uuid::Uuid::new_v4(), Some(9)),
        None
    );
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

fn browser_session_without_repo_binding(repo_id: RepoId, peer_id: PeerId) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    session.set_sync_scope_nonce(9);
    session.set_authenticated(peer_id);
    session
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
    let repo_id = repo_id.expect("writer binding RepoId");
    let handle = if readonly || branch.is_some() {
        DatabaseHandle::remote(
            db,
            branch.unwrap_or_else(|| PeerId::new("readonly")),
            repo_id,
            repo_name.into(),
        )
    } else {
        DatabaseHandle::local(repo_id, repo_name.into())
    };
    session.set_active_db(handle);
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
