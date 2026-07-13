//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Shared Axum/WebSocket runtime state.

use super::diff_projection::DiffProjectionExecutor;
use super::session::WsSession;
use super::source_control_grants::SourceControlWriteGrants;
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
    #[cfg(not(test))]
    pub(crate) diff_projection_executor: Arc<DiffProjectionExecutor>,
    #[cfg(not(test))]
    pub(crate) source_control_write_grants: Arc<SourceControlWriteGrants>,
}

impl AppState {
    #[cfg(not(test))]
    pub(crate) fn diff_projection_executor(&self) -> Arc<DiffProjectionExecutor> {
        self.diff_projection_executor.clone()
    }

    #[cfg(test)]
    pub(crate) fn diff_projection_executor(&self) -> Arc<DiffProjectionExecutor> {
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        static TEST_EXECUTORS: OnceLock<Mutex<HashMap<usize, Arc<DiffProjectionExecutor>>>> =
            OnceLock::new();
        let key = self as *const Self as usize;
        let stores = TEST_EXECUTORS.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            return Arc::new(DiffProjectionExecutor::new());
        };
        stores
            .entry(key)
            .or_insert_with(|| Arc::new(DiffProjectionExecutor::new()))
            .clone()
    }

    pub(crate) fn revoke_source_control_write_grant_for_session(&self, session: &WsSession) {
        if let Some(auth_session_id) = session.auth_session_id() {
            self.source_control_write_grants()
                .revoke_session(auth_session_id);
        }
    }

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
