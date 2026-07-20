use super::handlers::listing::handle_list_shadows;
use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use deve_core::config::SyncMode;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(tempfile::TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                deve_core::models::PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_missing_remote_catalog_reports_storage_corruption() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    std::fs::remove_dir_all(state.repo.remotes_dir())?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(77));
    session.switch_branch(Some("ghost-peer".into()));

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-scope".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(scope_nonce, Some(77));
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("Broken remote repo catalog")));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(
        session.active_branch.as_ref().map(|peer| peer.as_str()),
        Some("ghost-peer")
    );
    Ok(())
}
