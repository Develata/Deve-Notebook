use super::handle_switch_branch;
use crate::server::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_returns_to_last_local_repo_when_leaving_remote_scope() -> anyhow::Result<()>
{
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = crate::test_support::init_cataloged_repo_with_url(
        &ledger_dir,
        &projection_base,
        10,
        Some("urn:default".to_string()),
    )?
    .repo;
    let test_id = crate::test_support::init_cataloged_repo_with_url(
        &ledger_dir,
        &projection_base,
        10,
        Some("urn:test".to_string()),
    )?
    .repo_id;
    let peer_id = PeerId::new("peer-remote");
    let remote_repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: remote_repo_id,
            name: "default".into(),
            url: Some("urn:remote:default".into()),
        },
    )?;

    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(32);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_available: false,
        identity_key,
    });
    let (uni_tx, mut uni_rx) = mpsc::channel(32);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = browser_session(0);
    session.switch_repo(test_id.to_string(), Some(test_id));

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(1),
    )
    .await;
    while uni_rx.try_recv().is_ok() {}
    assert_eq!(
        session.active_branch.as_ref().map(|active| active.as_str()),
        Some(peer_id.as_str())
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(
        session.last_local_repo.as_deref(),
        Some(test_id.to_string().as_str())
    );
    assert_eq!(session.last_local_repo_id, Some(test_id));

    handle_switch_branch(&state, &ch, &mut session, None, Some(2)).await;
    while uni_rx.try_recv().is_ok() {}
    assert_eq!(session.active_branch, None);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(test_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}
