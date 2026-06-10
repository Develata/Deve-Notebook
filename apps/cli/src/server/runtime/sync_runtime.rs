//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Repo-scoped sync runtime assembly.

use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::{sync::SyncManager, sync::repo_scoped};
use std::path::Path;
use std::sync::Arc;

pub(crate) fn init_sync_manager(repo: Arc<RepoManager>) -> anyhow::Result<Arc<SyncManager>> {
    let sync_manager = Arc::new(SyncManager::new_checked(repo)?);
    sync_manager.scan()?;
    Ok(sync_manager)
}

pub(crate) fn build_sync_engine(
    peer_id: PeerId,
    repo: Arc<RepoManager>,
    sync_mode: SyncMode,
) -> Arc<RepoScopedSyncEngine> {
    Arc::new(repo_scoped::RepoScopedSyncEngine::new(
        peer_id, repo, sync_mode,
    ))
}

pub(crate) fn load_identity_key(host_dir: &Path) -> anyhow::Result<Arc<IdentityKeyPair>> {
    crate::server::security::load_or_generate_identity_key(host_dir)
}
