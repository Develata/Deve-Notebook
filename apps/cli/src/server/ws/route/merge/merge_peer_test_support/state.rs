use super::super::route_merge;
use crate::server::session::WsSession;
use crate::server::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ClientMessage;
use deve_core::sync::{SyncManager, repo_scoped::RepoScopedSyncEngine};
use std::path::Path;
use std::sync::Arc;

pub(crate) fn reopen_state(root: &Path) -> anyhow::Result<Arc<AppState>> {
    let vault = root.join("vault");
    let mut repo = RepoManager::init(root, 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&root.join("host"))?;
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(SyncManager::new(repo.clone(), vault)),
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
    }))
}

pub(crate) async fn request_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: &PeerId,
    doc_id: DocId,
    scope_nonce: u64,
) {
    route_merge(
        state,
        ch,
        session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(scope_nonce),
        },
    )
    .await;
}

pub(crate) fn ensure_remote_repo(
    state: &Arc<AppState>,
    repo_id: uuid::Uuid,
) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    Ok(peer_id)
}

pub(crate) fn browser_remote_session(
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session
}
