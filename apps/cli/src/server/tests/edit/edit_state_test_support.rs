//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

pub(crate) struct EditHarness {
    pub(crate) dir: TempDir,
    pub(crate) state: Arc<AppState>,
    pub(crate) default_repo_id: uuid::Uuid,
    pub(crate) test_repo_id: Option<uuid::Uuid>,
}

pub(crate) fn edit_harness(with_test_repo: bool) -> anyhow::Result<EditHarness> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    // Mirror production init order: establish the workspace identity marker before any
    // edit drives projection writeback, so edits exercise the real authority path
    // instead of failing closed on a missing identity gate.
    repo.ensure_local_repo_workspace_identity("default")?;
    let default_repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let test_repo_id = if with_test_repo {
        let mut test_repo = RepoManager::init(&ledger, 10, Some("test"), Some("urn:test"))?;
        test_repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        test_repo.ensure_local_repo_workspace_identity("test")?;
        Some(test_repo.get_repo_info()?.expect("test info").uuid)
    } else {
        None
    };
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(EditHarness {
        dir,
        default_repo_id,
        test_repo_id,
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
    })
}

pub(crate) fn seed_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    path: &str,
) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    Ok(doc_id)
}

pub(crate) fn seed_doc_with_content(
    state: &Arc<AppState>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<DocId> {
    let doc_id = seed_doc(state, repo_name, path)?;
    state.repo.append_generated_op_in_local_repo(
        repo_name,
        doc_id,
        PeerId::new("test-peer"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(doc_id)
}

pub(crate) fn unicast_channel(
    state: &Arc<AppState>,
) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}

pub(crate) fn writer_browser_session(
    repo_name: &str,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session.switch_repo(repo_name.into(), Some(repo_id));
    session.set_writer_identity(repo_id, PeerId::new("writer"), scope_nonce);
    session
}
