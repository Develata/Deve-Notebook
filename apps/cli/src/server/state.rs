//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Shared Axum/WebSocket runtime state.

use super::diff_projection::DiffProjectionExecutor;
use super::repo_mutation::RepoMutationPublicationGate;
#[cfg(test)]
use super::runtime::repo_lifecycle_runtime::RepoLifecycleCoordinator;
use super::runtime::repo_session_runtime::RepoSessionRuntime;
use super::runtime::watcher_runtime::WatcherRuntimeView;
use super::session::WsSession;
use super::source_control_grants::SourceControlWriteGrants;
use super::tree_state::RepoTreeRegistry;
use crate::remote_import_runtime::RemoteImportCoordinator;
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
#[cfg(not(test))]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

#[cfg(test)]
static TEST_WATCHER_VIEWS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, WatcherRuntimeView>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_MUTATION_GATES: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<RepoMutationPublicationGate>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_LIFECYCLE_COORDINATORS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<RepoLifecycleCoordinator>>>,
> = std::sync::OnceLock::new();
#[cfg(test)]
static TEST_LIFECYCLE_JOBS: std::sync::OnceLock<
    std::sync::Mutex<
        std::collections::HashMap<
            usize,
            Arc<super::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobRuntime>,
        >,
    >,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_REPO_SESSION_RUNTIMES: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<RepoSessionRuntime>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_REMOTE_IMPORT_COORDINATORS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<RemoteImportCoordinator>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_DIFF_PROJECTION_EXECUTORS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<DiffProjectionExecutor>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_SOURCE_CONTROL_WRITE_GRANTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, Arc<SourceControlWriteGrants>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
static TEST_AI_PROVIDER_SETTINGS: std::sync::OnceLock<
    std::sync::Mutex<
        std::collections::HashMap<
            usize,
            Arc<super::ai_chat::settings::NativeAiProviderSettingsRuntime>,
        >,
    >,
> = std::sync::OnceLock::new();

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
    #[cfg(not(test))]
    pub(crate) repo_mutation_gate: Arc<RepoMutationPublicationGate>,
    #[cfg(not(test))]
    pub(crate) watcher_runtime: WatcherRuntimeView,
    #[cfg(not(test))]
    pub(crate) remote_import: Arc<RemoteImportCoordinator>,
    #[cfg(not(test))]
    pub(crate) repo_sessions: Arc<RepoSessionRuntime>,
    #[cfg(not(test))]
    pub(crate) repo_lifecycle_jobs:
        Arc<super::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobRuntime>,
    #[cfg(not(test))]
    pub(crate) repo_creation_projection_base: Option<Arc<PathBuf>>,
    #[cfg(not(test))]
    pub(crate) ai_provider_settings: Arc<super::ai_chat::settings::NativeAiProviderSettingsRuntime>,
}

impl AppState {
    #[cfg(not(test))]
    pub(crate) fn ai_provider_settings(
        &self,
    ) -> Arc<super::ai_chat::settings::NativeAiProviderSettingsRuntime> {
        self.ai_provider_settings.clone()
    }

    #[cfg(test)]
    pub(crate) fn ai_provider_settings(
        &self,
    ) -> Arc<super::ai_chat::settings::NativeAiProviderSettingsRuntime> {
        let key = self as *const Self as usize;
        let runtimes = TEST_AI_PROVIDER_SETTINGS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let mut runtimes = runtimes.lock().expect("test AI provider settings");
        runtimes
            .entry(key)
            .or_insert_with(|| {
                Arc::new(
                    super::ai_chat::settings::NativeAiProviderSettingsRuntime::from_test_data_root(
                        self.repo
                            .ledger_dir()
                            .parent()
                            .expect("test ledger has data root"),
                    )
                    .expect("create test AI provider settings"),
                )
            })
            .clone()
    }

    #[cfg(not(test))]
    pub(crate) fn repo_creation_projection_base(&self) -> Option<&Path> {
        self.repo_creation_projection_base
            .as_deref()
            .map(PathBuf::as_path)
    }

    #[cfg(test)]
    pub(crate) fn repo_creation_projection_base(&self) -> Option<&std::path::Path> {
        None
    }

    pub(crate) fn catalog_membership_runtime(&self) -> deve_core::ledger::CatalogMembershipRuntime {
        let runtime = self.repo.catalog_membership_runtime();
        #[cfg(test)]
        self.repo
            .seed_catalog_membership_from_records()
            .expect("seed test catalog membership runtime");
        runtime
    }

    #[cfg(not(test))]
    pub(crate) fn repo_session_runtime(&self) -> Arc<RepoSessionRuntime> {
        self.repo_sessions.clone()
    }

    #[cfg(not(test))]
    pub(crate) fn repo_lifecycle_jobs(
        &self,
    ) -> Arc<super::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobRuntime> {
        self.repo_lifecycle_jobs.clone()
    }

    #[cfg(test)]
    pub(crate) fn repo_lifecycle_jobs(
        &self,
    ) -> Arc<super::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobRuntime> {
        use super::runtime::repo_lifecycle_job_runtime::{
            RepoLifecycleHostExecutor, RepoLifecycleHostPublicationSink, RepoLifecycleJobRuntime,
        };
        let key = self as *const Self as usize;
        let runtimes = TEST_LIFECYCLE_JOBS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let mut runtimes = runtimes.lock().expect("test lifecycle jobs");
        runtimes
            .entry(key)
            .or_insert_with(|| {
                RepoLifecycleJobRuntime::start(
                    self.repo.ledger_dir(),
                    Arc::new(RepoLifecycleHostExecutor::new(
                        self.repo_lifecycle_coordinator(),
                        self.repo.clone(),
                        self.watcher_runtime_view(),
                        self.sync_manager.clone(),
                        self.remote_import_coordinator(),
                    )),
                    Arc::new(RepoLifecycleHostPublicationSink::new(
                        self.repo.clone(),
                        self.watcher_runtime_view(),
                        self.repo_session_runtime(),
                        self.tx.clone(),
                    )),
                )
                .expect("start test repo lifecycle jobs")
            })
            .clone()
    }

    #[cfg(test)]
    pub(crate) fn repo_lifecycle_coordinator(&self) -> Arc<RepoLifecycleCoordinator> {
        let key = self as *const Self as usize;
        let stores = TEST_LIFECYCLE_COORDINATORS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let mut stores = stores.lock().expect("test repo lifecycle coordinators");
        stores
            .entry(key)
            .or_insert_with(|| {
                let membership = self.catalog_membership_runtime();
                for summary in self
                    .repo
                    .list_cataloged_local_repo_summaries()
                    .expect("test Remote Import startup repo list")
                {
                    deve_core::remote_import::RemoteImportService::recover_startup(
                        &self.repo,
                        summary.repo_id,
                    )
                    .expect("test Remote Import startup recovery");
                }
                let starts = super::setup::file_watcher_starts(self.sync_manager.clone())
                    .expect("test watcher starts");
                let supervisor = Arc::new(
                    super::runtime::watcher_runtime::WatcherSupervisor::start_all(
                        starts,
                        Arc::new(|_| {}),
                    )
                    .expect("test watcher supervisor"),
                );
                self.set_watcher_runtime_view_for_test(supervisor.view());
                let gate = self.repo_mutation_gate();
                RepoLifecycleCoordinator::new(
                    self.repo.clone(),
                    self.sync_manager.clone(),
                    gate,
                    supervisor,
                    self.remote_import_coordinator(),
                    membership,
                    None,
                )
            })
            .clone()
    }

    #[cfg(test)]
    pub(crate) fn repo_session_runtime(&self) -> Arc<RepoSessionRuntime> {
        let key = self as *const Self as usize;
        let runtimes = TEST_REPO_SESSION_RUNTIMES
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let mut runtimes = runtimes.lock().expect("test repo session runtimes");
        runtimes
            .entry(key)
            .or_insert_with(|| {
                let membership = self.catalog_membership_runtime();
                RepoSessionRuntime::new(membership)
            })
            .clone()
    }

    #[cfg(not(test))]
    pub(crate) fn remote_import_coordinator(&self) -> Arc<RemoteImportCoordinator> {
        self.remote_import.clone()
    }

    #[cfg(test)]
    pub(crate) fn remote_import_coordinator(&self) -> Arc<RemoteImportCoordinator> {
        let key = self as *const Self as usize;
        let stores = TEST_REMOTE_IMPORT_COORDINATORS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            return Arc::new(RemoteImportCoordinator::new(
                self.repo.clone(),
                self.sync_manager.clone(),
                self.catalog_membership_runtime(),
            ));
        };
        stores
            .entry(key)
            .or_insert_with(|| {
                Arc::new(RemoteImportCoordinator::new(
                    self.repo.clone(),
                    self.sync_manager.clone(),
                    self.catalog_membership_runtime(),
                ))
            })
            .clone()
    }

    #[cfg(not(test))]
    pub(crate) fn watcher_runtime_view(&self) -> WatcherRuntimeView {
        self.watcher_runtime.clone()
    }

    #[cfg(not(test))]
    pub(crate) fn diff_projection_executor(&self) -> Arc<DiffProjectionExecutor> {
        self.diff_projection_executor.clone()
    }

    #[cfg(test)]
    pub(crate) fn diff_projection_executor(&self) -> Arc<DiffProjectionExecutor> {
        let key = self as *const Self as usize;
        let stores = TEST_DIFF_PROJECTION_EXECUTORS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
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
        let key = self as *const Self as usize;
        let stores = TEST_SOURCE_CONTROL_WRITE_GRANTS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            return Arc::new(SourceControlWriteGrants::new());
        };
        stores
            .entry(key)
            .or_insert_with(|| Arc::new(SourceControlWriteGrants::new()))
            .clone()
    }

    #[cfg(not(test))]
    pub(crate) fn repo_mutation_gate(&self) -> Arc<RepoMutationPublicationGate> {
        debug_assert!(
            self.repo_mutation_gate
                .uses_watcher_runtime(&self.watcher_runtime),
            "AppState watcher view and mutation gate must share one runtime"
        );
        self.repo_mutation_gate.clone()
    }

    #[cfg(test)]
    pub(crate) fn repo_mutation_gate(&self) -> Arc<RepoMutationPublicationGate> {
        let key = self as *const Self as usize;
        let stores = TEST_MUTATION_GATES
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            panic!("test mutation gate registry is poisoned");
        };
        stores
            .entry(key)
            .or_insert_with(|| {
                Arc::new(RepoMutationPublicationGate::new(
                    self.watcher_runtime_view(),
                    self.repo
                        .claim_repo_catalog_cut_authority()
                        .expect("claim test repo catalog cut authority"),
                ))
            })
            .clone()
    }

    #[cfg(test)]
    pub(crate) fn watcher_runtime_view(&self) -> WatcherRuntimeView {
        let key = self as *const Self as usize;
        let stores = TEST_WATCHER_VIEWS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        let Ok(mut stores) = stores.lock() else {
            return WatcherRuntimeView::permissive_for_tests();
        };
        stores
            .entry(key)
            .or_insert_with(WatcherRuntimeView::permissive_for_tests)
            .clone()
    }

    #[cfg(test)]
    pub(crate) fn set_watcher_runtime_view_for_test(&self, view: WatcherRuntimeView) {
        let key = self as *const Self as usize;
        if let Ok(mut stores) = TEST_WATCHER_VIEWS
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
        {
            stores.insert(key, view);
        }
        if let Ok(mut gates) = TEST_MUTATION_GATES
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
        {
            gates.remove(&key);
        }
    }
}

#[cfg(test)]
impl Drop for AppState {
    fn drop(&mut self) {
        let key = self as *const Self as usize;
        if let Some(stores) = TEST_WATCHER_VIEWS.get()
            && let Ok(mut stores) = stores.lock()
        {
            stores.remove(&key);
        }
        if let Some(gates) = TEST_MUTATION_GATES.get()
            && let Ok(mut gates) = gates.lock()
        {
            gates.remove(&key);
        }
        if let Some(coordinators) = TEST_LIFECYCLE_COORDINATORS.get()
            && let Ok(mut coordinators) = coordinators.lock()
            && let Some(coordinator) = coordinators.remove(&key)
        {
            coordinator.shutdown_watchers_for_test();
        }
        if let Some(runtimes) = TEST_LIFECYCLE_JOBS.get()
            && let Ok(mut runtimes) = runtimes.lock()
        {
            runtimes.remove(&key);
        }
        if let Some(runtimes) = TEST_REPO_SESSION_RUNTIMES.get()
            && let Ok(mut runtimes) = runtimes.lock()
        {
            runtimes.remove(&key);
        }
        if let Some(coordinators) = TEST_REMOTE_IMPORT_COORDINATORS.get()
            && let Ok(mut coordinators) = coordinators.lock()
        {
            coordinators.remove(&key);
        }
        if let Some(executors) = TEST_DIFF_PROJECTION_EXECUTORS.get()
            && let Ok(mut executors) = executors.lock()
        {
            executors.remove(&key);
        }
        if let Some(grants) = TEST_SOURCE_CONTROL_WRITE_GRANTS.get()
            && let Ok(mut grants) = grants.lock()
        {
            grants.remove(&key);
        }
        if let Some(runtimes) = TEST_AI_PROVIDER_SETTINGS.get()
            && let Ok(mut runtimes) = runtimes.lock()
        {
            runtimes.remove(&key);
        }
    }
}
