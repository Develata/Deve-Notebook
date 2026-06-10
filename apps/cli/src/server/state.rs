//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Shared Axum/WebSocket runtime state.

use super::source_control_grants::SourceControlWriteGrants;
use super::tree_state::RepoTreeRegistry;
use deve_core::config::GitBridgeMode;
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
    pub git_bridge: GitBridgeMode,
    #[cfg(not(test))]
    pub source_control_write_grants: Arc<SourceControlWriteGrants>,
}

impl AppState {
    #[cfg(not(test))]
    pub(crate) fn source_control_write_grants(&self) -> Arc<SourceControlWriteGrants> {
        self.source_control_write_grants.clone()
    }

    #[cfg(test)]
    pub(crate) fn source_control_write_grants(&self) -> Arc<SourceControlWriteGrants> {
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        static TEST_GRANTS: OnceLock<Mutex<HashMap<usize, Arc<SourceControlWriteGrants>>>> =
            OnceLock::new();
        let key = self as *const Self as usize;
        let stores = TEST_GRANTS.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            return Arc::new(SourceControlWriteGrants::new());
        };
        stores
            .entry(key)
            .or_insert_with(|| Arc::new(SourceControlWriteGrants::new()))
            .clone()
    }
}
