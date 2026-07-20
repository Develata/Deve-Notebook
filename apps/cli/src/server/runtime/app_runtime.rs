//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Shared AppState and tree runtime assembly.

#[cfg(not(test))]
use crate::remote_import_runtime::RemoteImportCoordinator;
#[cfg(not(test))]
use crate::server::diff_projection::DiffProjectionExecutor;
#[cfg(not(test))]
use crate::server::repo_mutation::RepoMutationPublicationGate;
#[cfg(not(test))]
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleHostExecutor, RepoLifecycleHostPublicationSink, RepoLifecycleJobRuntime,
};
#[cfg(not(test))]
use crate::server::runtime::repo_lifecycle_runtime::RepoLifecycleCoordinator;
#[cfg(not(test))]
use crate::server::runtime::repo_session_runtime::RepoSessionRuntime;
use crate::server::runtime::watcher_runtime::{WatcherRuntimeView, WatcherSupervisor};
#[cfg(not(test))]
use crate::server::source_control_grants::SourceControlWriteGrants;
use crate::server::{AppState, tree_state::RepoTreeRegistry};
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::{SyncManager, repo_scoped::RepoScopedSyncEngine};
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) fn new_server_broadcast_channel() -> broadcast::Sender<ServerMessage> {
    let (tx, _rx) = broadcast::channel(100);
    tx
}

pub(crate) fn build_tree_registry() -> Arc<RepoTreeRegistry> {
    Arc::new(RepoTreeRegistry::new())
}

pub(crate) struct AppStateParts {
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub plugins: Vec<Box<dyn PluginRuntime>>,
    pub sync_engine: Arc<RepoScopedSyncEngine>,
    pub tree_manager: Arc<RepoTreeRegistry>,
    #[cfg(feature = "search")]
    pub search_available: bool,
    pub identity_key: Arc<IdentityKeyPair>,
    pub watcher_runtime: WatcherRuntimeView,
    pub watcher_supervisor: Arc<WatcherSupervisor>,
}

pub(crate) fn build_app_state(parts: AppStateParts) -> anyhow::Result<Arc<AppState>> {
    #[cfg(test)]
    let _watcher_runtime = parts.watcher_runtime;
    #[cfg(test)]
    let _watcher_supervisor = parts.watcher_supervisor;
    #[cfg(not(test))]
    let catalog_membership = parts.repo.catalog_membership_runtime();
    #[cfg(not(test))]
    let remote_import = Arc::new(RemoteImportCoordinator::new(
        parts.repo.clone(),
        parts.sync_manager.clone(),
        catalog_membership.clone(),
    ));
    #[cfg(not(test))]
    let repo_sessions = RepoSessionRuntime::new(catalog_membership);
    #[cfg(not(test))]
    let repo_mutation_gate = Arc::new(RepoMutationPublicationGate::new(
        parts.watcher_runtime.clone(),
        parts.repo.claim_repo_catalog_cut_authority()?,
    ));
    #[cfg(not(test))]
    let repo_lifecycle = RepoLifecycleCoordinator::new(
        parts.repo.clone(),
        parts.sync_manager.clone(),
        repo_mutation_gate.clone(),
        parts.watcher_supervisor.clone(),
        remote_import.clone(),
        parts.repo.catalog_membership_runtime(),
    );
    #[cfg(not(test))]
    let repo_lifecycle_jobs = RepoLifecycleJobRuntime::start(
        parts.repo.ledger_dir(),
        Arc::new(RepoLifecycleHostExecutor::new(
            repo_lifecycle.clone(),
            parts.repo.clone(),
            parts.watcher_runtime.clone(),
        )),
        Arc::new(RepoLifecycleHostPublicationSink::new(
            parts.repo.clone(),
            parts.watcher_runtime.clone(),
            repo_sessions.clone(),
            parts.tx.clone(),
        )),
    )?;
    Ok(Arc::new(AppState {
        repo: parts.repo,
        sync_manager: parts.sync_manager,
        tx: parts.tx,
        plugins: parts.plugins,
        sync_engine: parts.sync_engine,
        tree_manager: parts.tree_manager,
        #[cfg(feature = "search")]
        search_available: parts.search_available,
        identity_key: parts.identity_key,
        #[cfg(not(test))]
        watcher_runtime: parts.watcher_runtime.clone(),
        #[cfg(not(test))]
        diff_projection_executor: Arc::new(DiffProjectionExecutor::new()),
        #[cfg(not(test))]
        source_control_write_grants: Arc::new(SourceControlWriteGrants::new()),
        #[cfg(not(test))]
        repo_mutation_gate,
        #[cfg(not(test))]
        remote_import,
        #[cfg(not(test))]
        repo_sessions,
        #[cfg(not(test))]
        repo_lifecycle_jobs,
    }))
}
