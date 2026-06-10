//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Shared AppState and tree runtime assembly.

use crate::server::{AppState, tree_state::RepoTreeRegistry};
use deve_core::config::GitBridgeMode;
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
    pub git_bridge: GitBridgeMode,
}

pub(crate) fn build_app_state(parts: AppStateParts) -> Arc<AppState> {
    Arc::new(AppState {
        repo: parts.repo,
        sync_manager: parts.sync_manager,
        tx: parts.tx,
        plugins: parts.plugins,
        sync_engine: parts.sync_engine,
        tree_manager: parts.tree_manager,
        #[cfg(feature = "search")]
        search_available: parts.search_available,
        identity_key: parts.identity_key,
        git_bridge: parts.git_bridge,
    })
}
