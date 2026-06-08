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

pub(crate) fn build_app_state(
    repo: Arc<RepoManager>,
    sync_manager: Arc<SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
    plugins: Vec<Box<dyn PluginRuntime>>,
    sync_engine: Arc<RepoScopedSyncEngine>,
    tree_manager: Arc<RepoTreeRegistry>,
    #[cfg(feature = "search")] search_available: bool,
    identity_key: Arc<IdentityKeyPair>,
    git_bridge: GitBridgeMode,
) -> Arc<AppState> {
    Arc::new(AppState {
        repo,
        sync_manager,
        tx,
        plugins,
        sync_engine,
        tree_manager,
        #[cfg(feature = "search")]
        search_available,
        identity_key,
        git_bridge,
    })
}
