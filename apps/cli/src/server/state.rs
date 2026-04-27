//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Shared Axum/WebSocket runtime state.

use super::tree_state::RepoTreeRegistry;
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppState {
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub plugins: Vec<Box<dyn PluginRuntime>>,
    pub sync_engine: Arc<RepoScopedSyncEngine>,
    /// Repo-scoped 文件树状态。
    pub tree_manager: Arc<RepoTreeRegistry>,
    #[cfg(feature = "search")]
    pub search_available: bool,
    pub identity_key: Arc<deve_core::security::IdentityKeyPair>,
}
