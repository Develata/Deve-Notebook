//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 04_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoManager, database::DatabaseHandle};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::{path::PathBuf, sync::Arc};
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

pub(crate) struct DocsHarness {
    pub(crate) dir: TempDir,
    pub(crate) state: Arc<AppState>,
    pub(crate) repo_id: uuid::Uuid,
}

impl DocsHarness {
    pub(crate) fn workspace_path(&self, relative: &str) -> PathBuf {
        self.state
            .repo
            .local_repo_workspace_path("default", relative)
            .expect("workspace path")
    }
}

pub(crate) fn docs_harness() -> anyhow::Result<DocsHarness> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(DocsHarness {
        dir,
        state: Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
        repo_id,
    })
}

pub(crate) fn channel(state: &Arc<AppState>) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}

pub(crate) fn local_session(state: &Arc<AppState>, repo_id: uuid::Uuid) -> WsSession {
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));
    session
}

pub(crate) fn browser_session(
    state: &Arc<AppState>,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> WsSession {
    let mut session = local_session(state, repo_id);
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(crate) fn stale_browser_scope_session(
    state: &Arc<AppState>,
    bound_repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(
        state.repo.local_repo_name().to_string(),
        Some(uuid::Uuid::new_v4()),
    );
    session.set_scope_nonce(Some(scope_nonce));
    session.bind_repo(bound_repo_id);
    session
}

pub(crate) fn stale_db_handle(
    path: PathBuf,
    readonly: bool,
    branch: Option<PeerId>,
    repo_name: &str,
) -> anyhow::Result<DatabaseHandle> {
    Ok(DatabaseHandle {
        db: Arc::new(redb::Database::create(path)?),
        readonly,
        branch,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: repo_name.into(),
    })
}

pub(crate) async fn recv_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (ServerError, Option<u64>) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => (error, scope_nonce),
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}
