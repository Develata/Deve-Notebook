//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use crate::server::{
    handlers::sync,
    sync_delete_peer_test_support::{
        active_peer_session, assert_remote_scope_cleared, assert_shadow_list, browser_session,
        ensure_shadow_repo,
    },
    sync_scope_cleanup_test_support::{build_state, unicast_channel},
};
use deve_core::models::PeerId;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_delete_peer_returns_scoped_shadow_list() -> anyhow::Result<()> {
    let (_dir, state, _) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    ensure_shadow_repo(&state, &peer_id, uuid::Uuid::new_v4())?;

    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(11);

    sync::handle_delete_peer(&state, &ch, &mut session, peer_id.to_string()).await;

    assert_shadow_list(&mut uni_rx, 11, true).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_active_peer_clears_stale_remote_bindings() -> anyhow::Result<()> {
    let (_dir, state, _) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    ensure_shadow_repo(&state, &peer_id, repo_id)?;

    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = active_peer_session(&state, &peer_id, repo_id, 15, peer_id.clone())?;

    sync::handle_delete_peer(&state, &ch, &mut session, peer_id.to_string()).await;

    assert_shadow_list(&mut uni_rx, 15, false).await;
    assert_remote_scope_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_active_peer_clears_browser_sync_binding_even_when_auth_peer_differs()
-> anyhow::Result<()> {
    let (_dir, state, _) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    ensure_shadow_repo(&state, &peer_id, repo_id)?;

    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session =
        active_peer_session(&state, &peer_id, repo_id, 21, PeerId::new("browser-writer"))?;

    sync::handle_delete_peer(&state, &ch, &mut session, peer_id.to_string()).await;

    assert_shadow_list(&mut uni_rx, 21, false).await;
    assert_remote_scope_cleared(&session);
    Ok(())
}
