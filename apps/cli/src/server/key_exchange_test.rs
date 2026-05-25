//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::key_exchange::handle_request_key;
use super::key_exchange_test_support::{
    assert_key_provide, bound_browser_session, browser_session, build_state, ensure_shadow_notes,
    recv_key_denied, recv_protocol_error, remote_browser_session, unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_rejects_non_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = super::session::WsSession::new();
    session.switch_repo("notes".into(), Some(repo_id));
    session.bind_repo(repo_id);

    handle_request_key(&state, &ch, &mut session).await;

    let (code, scope_nonce) = recv_protocol_error(&mut uni_rx).await;
    assert_eq!(code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_uses_current_browser_scope_when_sync_scope_is_stale() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = bound_browser_session(repo_id, 17);
    session.set_sync_scope_nonce(9);

    handle_request_key(&state, &ch, &mut session).await;

    assert_key_provide(&mut uni_rx, repo_id, 17, None).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_on_remote_branch_uses_local_counterpart_keys_root() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    ensure_shadow_notes(&state, &peer_id, repo_id, "urn:test:notes")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = remote_browser_session(&peer_id, repo_id, 21);
    session.set_sync_scope_nonce(5);

    handle_request_key(&state, &ch, &mut session).await;

    assert_key_provide(&mut uni_rx, repo_id, 21, Some(peer_id.clone())).await;
    assert!(state.repo.local_repo_notegit_keys_root("notes")?.exists());
    let notes_root = state.repo.local_repo_workspace_root("notes")?;
    let projection_base = notes_root.parent().expect("repo root must have projection base");
    let shadow_keys = deve_core::utils::notegit::repo_keys_dir(
        &projection_base.join("shadow-notes"),
    );
    assert!(!shadow_keys.exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_denies_corrupt_repo_key() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let key_path = state
        .repo
        .local_repo_notegit_keys_root("notes")?
        .join("repo.key");
    std::fs::create_dir_all(
        key_path
            .parent()
            .expect("repo.key must have a parent directory"),
    )?;
    std::fs::write(&key_path, [1, 2, 3, 4])?;

    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = bound_browser_session(repo_id, 31);
    session.set_sync_scope_nonce(9);

    handle_request_key(&state, &ch, &mut session).await;

    let denied = recv_key_denied(&mut uni_rx).await;
    assert_eq!(denied.repo_id, Some(repo_id));
    assert_eq!(denied.scope_nonce, 31);
    assert_eq!(denied.branch, None);
    assert_eq!(denied.error.code, ServerErrorCode::StoragePersistFailed);
    assert!(denied
        .error
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("Corrupt repo key")));
    assert_eq!(std::fs::read(key_path)?, vec![1, 2, 3, 4]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_on_missing_shadow_branch_clears_remote_scope() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(41);
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    handle_request_key(&state, &ch, &mut session).await;

    let denied = recv_key_denied(&mut uni_rx).await;
    assert_eq!(denied.repo_id, None);
    assert_eq!(denied.scope_nonce, 41);
    assert_eq!(denied.branch, None);
    assert_eq!(denied.error.code, ServerErrorCode::ScStaleScope);
    assert!(denied
        .error
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("Remote branch not available:")));
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
