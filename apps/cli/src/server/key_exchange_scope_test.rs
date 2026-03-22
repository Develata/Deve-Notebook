use super::handlers::key_exchange::handle_request_key;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let host_dir = dir.path().join("host");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(8).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: security::load_or_generate_identity_key(&host_dir)?,
        }),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_denies_remote_scope_when_only_url_matches_local_repo() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let shadow_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: shadow_id,
            name: "shadow-notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(51));
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(shadow_id));
    session.bind_repo(shadow_id);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        }) => {
            assert_eq!(repo_id, Some(shadow_id));
            assert_eq!(scope_nonce, 51);
            assert_eq!(branch, Some(peer_id));
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(error.detail.as_deref().is_some_and(|detail| {
                detail.contains("No local writable repo available for current scope")
            }));
        }
        other => panic!("expected KeyDenied, got {:?}", other),
    }
    Ok(())
}
