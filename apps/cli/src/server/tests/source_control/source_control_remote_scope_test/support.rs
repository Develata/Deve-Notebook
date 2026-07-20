//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use crate::server::{security, tree_state::RepoTreeRegistry, AppState};
use deve_core::config::SyncMode;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::models::{serialize_ledger_entry, DocId, LedgerEntry, Op, PeerId};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let test_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "test",
        &projection_base,
        10,
        Some("urn:test"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
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
        test_id,
    ))
}

pub(super) fn seed_shadow_doc(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
) -> anyhow::Result<DocId> {
    seed_shadow_doc_with_url(repo, peer_id, repo_id, "shadow-notes", "urn:test")
}

pub(super) fn seed_shadow_doc_with_url(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    repo_name: &str,
    repo_url: &str,
) -> anyhow::Result<DocId> {
    repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: repo_name.into(),
            url: Some(repo_url.into()),
        },
    )?;
    let doc_id = DocId::new();
    repo.run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
        let _ = deve_core::ledger::node_meta::ensure_file_node(db, "notes/a.md", doc_id)?;
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "remote".into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        );
        let bytes = serialize_ledger_entry(&entry)?;
        let write = db.begin_write()?;
        write
            .open_table(LEDGER_OPS)?
            .insert(1u64, bytes.as_slice())?;
        write
            .open_multimap_table(DOC_OPS)?
            .insert(doc_id.as_u128(), 1u64)?;
        write.commit()?;
        Ok(())
    })?;
    Ok(doc_id)
}
