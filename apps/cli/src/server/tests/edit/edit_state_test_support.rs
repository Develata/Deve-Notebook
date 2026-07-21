//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use crate::repo_init::{
    initialize_initial_local_repo_workspace, prepare_local_repo_workspace_with_owner,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, FactActor, Op, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

pub(crate) struct EditHarness {
    pub(crate) dir: TempDir,
    pub(crate) state: Arc<AppState>,
    pub(crate) default_repo_id: uuid::Uuid,
    pub(crate) default_repo_name: String,
    pub(crate) test_repo_id: Option<uuid::Uuid>,
    pub(crate) test_repo_name: Option<String>,
}

pub(crate) fn edit_harness(with_test_repo: bool) -> anyhow::Result<EditHarness> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let default_repo_id = uuid::Uuid::new_v4();
    initialize_initial_local_repo_workspace(
        &ledger,
        "default",
        &projection_base,
        10,
        Some(default_repo_id),
        Some("urn:default".to_string()),
    )?;
    let repo = RepoManager::init_existing_for_repo_id(&ledger, 10, default_repo_id)?;
    repo.seed_catalog_membership_from_records()?;
    let default_repo_name = default_repo_id.to_string();
    let (test_repo_id, test_repo_name) = if with_test_repo {
        let test_repo_id = uuid::Uuid::new_v4();
        let prepared_authority = prepare_local_repo_workspace_with_owner(
            &repo,
            test_repo_id,
            &projection_base,
            Some("urn:test".to_string()),
        )?;
        let authority = repo.claim_repo_catalog_cut_authority()?;
        let prepared = repo.prepare_repo_creation_membership_with_authority(
            test_repo_id,
            uuid::Uuid::new_v4(),
            &prepared_authority,
        )?;
        let revalidated = repo.revalidate_repo_creation_membership_with_authority(
            &prepared,
            &prepared_authority,
        )?;
        let permit = authority.permit(test_repo_id)?;
        let committed =
            repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
        repo.activate_prepared_local_repo_authority(
            prepared_authority,
            &prepared,
            &committed,
        )?;
        repo.host_repo_alias_runtime()
            .set_alias(test_repo_id, "test", 0)?;
        (Some(test_repo_id), Some(test_repo_id.to_string()))
    } else {
        (None, None)
    };
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(EditHarness {
        dir,
        default_repo_id,
        default_repo_name,
        test_repo_id,
        test_repo_name,
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
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            repo_name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
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
