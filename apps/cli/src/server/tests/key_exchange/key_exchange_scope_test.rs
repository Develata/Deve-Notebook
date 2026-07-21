//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::key_exchange::handle_request_key;
use super::key_exchange_test_support::{
    assert_key_provide, browser_session, build_state, ensure_shadow_notes, recv_key_denied,
    remote_browser_session, unicast_channel,
};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_denies_remote_scope_when_only_url_matches_local_repo() -> anyhow::Result<()> {
    let (_dir, state, _) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let shadow_id = uuid::Uuid::new_v4();
    ensure_shadow_notes(&state, &peer_id, shadow_id, "urn:test:notes")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = remote_browser_session(&peer_id, shadow_id, 51);

    handle_request_key(&state, &ch, &mut session).await;

    let denied = recv_key_denied(&mut uni_rx).await;
    assert_eq!(denied.repo_id, Some(shadow_id));
    assert_eq!(denied.scope_nonce, 51);
    assert_eq!(denied.branch, Some(peer_id));
    assert_eq!(denied.error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(denied.error.detail.as_deref().is_some_and(|detail| {
        detail.contains("No local writable repo available for current scope")
    }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(61);

    handle_request_key(&state, &ch, &mut session).await;

    assert_key_provide(&mut uni_rx, repo_id, 61, None).await;
    assert_eq!(
        session.active_repo.as_deref(),
        Some(repo_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(repo_id));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_with_stale_local_binding_bootstraps_single_repo() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    let stale_db = Arc::new(redb::Database::create(dir.path().join("stale-local.redb"))?);
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(71);
    drop(stale_db);
    session.set_active_db(DatabaseHandle::local(
        uuid::Uuid::new_v4(),
        "ghost".into(),
    ));
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(71);

    handle_request_key(&state, &ch, &mut session).await;

    assert_key_provide(&mut uni_rx, repo_id, 71, None).await;
    assert_eq!(
        session.active_repo.as_deref(),
        Some(repo_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(repo_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
