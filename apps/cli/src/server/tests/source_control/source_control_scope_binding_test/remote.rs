//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::build_state;
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_changes, session::WsSession,
    source_control_grants::AuthSessionId,
};
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_changes_without_repo_selection_report_stale_remote_scope() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let repo_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test:shadow-notes".into()),
        },
    )?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(13);

    handle_get_changes(&state, &ch, &mut session, Some("req-remote-miss".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.starts_with("stale remote scope:"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&peer_id));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn source_control_scope_cleanup_revokes_write_grant() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let repo_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test:shadow-notes".into()),
        },
    )?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let auth_session_id = AuthSessionId::for_test("source-control-scope-cleanup");
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.bind_auth_session(auth_session_id.clone());
    session.set_scope_nonce(Some(13));
    session.switch_branch(Some(peer_id.to_string()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(13);
    state.source_control_write_grants().grant(
        auth_session_id.clone(),
        repo_id,
        PeerId::new("writer"),
        13,
    );
    assert!(
        state
            .source_control_write_grants()
            .authorize(&auth_session_id, repo_id, 13)
            .is_ok()
    );

    handle_get_changes(
        &state,
        &ch,
        &mut session,
        Some("req-remote-miss".into()),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    let error = state
        .source_control_write_grants()
        .authorize(&auth_session_id, repo_id, 13)
        .expect_err("grant must be revoked when source control clears scope binding");
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_changes_on_missing_branch_clears_stale_scope() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(default_id));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(29);

    handle_get_changes(&state, &ch, &mut session, Some("req-remote-gone".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|d| d.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
