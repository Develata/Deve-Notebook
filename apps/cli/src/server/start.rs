//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! HTTP/WebSocket server boot sequence.

use super::{plugin_host, runtime};
use crate::server::launch::ServerLaunchOptions;
use deve_core::config::{AppProfile, P2pConfig, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

mod shutdown_signal;
mod transport_shutdown;

use transport_shutdown::{
    RuntimeShutdownDeadline, deadline_after, remaining_shutdown_budget,
    serve_router_until_shutdown_with_deadline,
};

const SERVER_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct ServerTransportRuntime {
    app_state: Arc<super::AppState>,
    profile: AppProfile,
    host_dir: Arc<PathBuf>,
    p2p_inbound_token_env: Option<String>,
}

pub(crate) struct ServerTransportServeError {
    source: anyhow::Error,
    sessions_retired: bool,
}

impl ServerTransportServeError {
    pub(crate) fn sessions_retired(&self) -> bool {
        self.sessions_retired
    }

    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        self.source
    }
}

impl std::fmt::Display for ServerTransportServeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::fmt::Debug for ServerTransportServeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerTransportServeError")
            .field("source", &self.source)
            .field("sessions_retired", &self.sessions_retired)
            .finish()
    }
}

impl std::error::Error for ServerTransportServeError {}

pub(crate) struct EmbeddedServerRuntime {
    transport: ServerTransportRuntime,
    _native_ai_registration: super::ai_chat::NativeAiRuntimeRegistration,
    _ai_provider_settings_registration: super::ai_chat::settings::ProviderSettingsRegistration,
    background_tasks: Option<runtime::BackgroundRuntimeTasks>,
    watcher_supervisor: Option<Arc<runtime::watcher_runtime::WatcherSupervisor>>,
    repo_lifecycle_jobs: Option<Arc<runtime::repo_lifecycle_job_runtime::RepoLifecycleJobRuntime>>,
}

impl EmbeddedServerRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn initialize(
        repo: Arc<RepoManager>,
        launch: &ServerLaunchOptions,
        plugins: Vec<Box<dyn PluginRuntime>>,
        #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
        sync_mode: SyncMode,
        p2p: P2pConfig,
        prewarm_enabled: bool,
    ) -> anyhow::Result<Self> {
        runtime::install_repo_host_apis(&repo)?;
        let data_root = match launch.ai_provider_settings_data_root() {
            Some(platform_root) => platform_root,
            None => repo
                .ledger_dir()
                .parent()
                .ok_or_else(|| anyhow::anyhow!("ledger directory has no data root"))?,
        };
        let ai_provider_settings = Arc::new(
            super::ai_chat::settings::NativeAiProviderSettingsRuntime::from_data_root(data_root)?,
        );
        let ai_provider_settings_registration =
            super::ai_chat::settings::register(ai_provider_settings.clone())?;
        runtime::init_observability_runtime()?;
        let host_dir = runtime::prepare_host_layout(repo.as_ref())?;
        let tx = runtime::new_server_broadcast_channel();

        let sync_manager = runtime::init_sync_manager(repo.clone())?;
        runtime::install_sync_host_api(sync_manager.clone())?;
        let local_repos = repo.list_cataloged_local_repo_summaries()?;
        repo.seed_catalog_membership_from_records()?;
        for summary in &local_repos {
            deve_core::remote_import::RemoteImportService::recover_startup(&repo, summary.repo_id)?;
        }

        #[cfg(feature = "search")]
        let search_available = runtime::search_available(profile);

        let key_pair = runtime::load_identity_key(&host_dir)?;
        let peer_id = key_pair.peer_id();
        tracing::info!("Server PeerID: {}", peer_id);

        let sync_engine = build_sync_engine(peer_id, repo.clone(), sync_mode);
        let tree_manager = runtime::build_tree_registry();
        let watcher_supervisor = Arc::new(runtime::start_file_watchers(
            sync_manager.clone(),
            tx.clone(),
        )?);
        let watcher_runtime = watcher_supervisor.view();
        let app_state = runtime::build_app_state(runtime::AppStateParts {
            repo: repo.clone(),
            sync_manager: sync_manager.clone(),
            tx: tx.clone(),
            plugins,
            sync_engine,
            tree_manager,
            #[cfg(feature = "search")]
            search_available,
            identity_key: key_pair,
            watcher_runtime,
            watcher_supervisor: watcher_supervisor.clone(),
            repo_creation_projection_base: launch
                .repo_creation_projection_base()
                .map(std::path::Path::to_path_buf),
            #[cfg(not(test))]
            ai_provider_settings,
        })?;
        let native_ai_registration =
            super::ai_chat::NativeAiRuntimeRegistration::from_plugins(&app_state.plugins);
        #[cfg(not(test))]
        deve_core::plugin::runtime::host::set_managed_note_mutation_host(Arc::new(
            crate::server::repo_mutation::CliManagedNoteMutationHost::new(&app_state),
        ))?;
        #[cfg(not(test))]
        deve_core::plugin::runtime::host::set_managed_source_control_mutation_host(Arc::new(
            crate::server::repo_mutation::CliManagedSourceControlMutationHost::new(&app_state),
        ))?;
        let p2p_inbound_token_env = p2p.inbound_token_env.clone();
        let background_tasks =
            runtime::spawn_background_runtime_tasks(p2p, app_state.clone(), repo, prewarm_enabled);

        let repo_lifecycle_jobs = app_state.repo_lifecycle_jobs();
        Ok(Self {
            transport: ServerTransportRuntime {
                app_state,
                profile,
                host_dir: Arc::new(host_dir),
                p2p_inbound_token_env,
            },
            _native_ai_registration: native_ai_registration,
            _ai_provider_settings_registration: ai_provider_settings_registration,
            background_tasks: Some(background_tasks),
            watcher_supervisor: Some(watcher_supervisor),
            repo_lifecycle_jobs: Some(repo_lifecycle_jobs),
        })
    }

    pub(crate) fn transport(&self) -> ServerTransportRuntime {
        self.transport.clone()
    }

    pub(crate) async fn shutdown(self, timeout: Duration) -> anyhow::Result<()> {
        self.shutdown_until(deadline_after(timeout)).await
    }

    async fn shutdown_until(mut self, deadline: tokio::time::Instant) -> anyhow::Result<()> {
        let background_result = match self.background_tasks.take() {
            Some(tasks) => {
                tasks
                    .shutdown(remaining_shutdown_budget(
                        deadline,
                        tokio::time::Instant::now(),
                    ))
                    .await
            }
            None => Ok(()),
        };
        let lifecycle_result = match self.repo_lifecycle_jobs.take() {
            Some(runtime) => runtime
                .shutdown_with_timeout(remaining_shutdown_budget(
                    deadline,
                    tokio::time::Instant::now(),
                ))
                .await
                .map_err(anyhow::Error::from),
            None => Ok(()),
        };
        let watcher_result = match self.watcher_supervisor.take() {
            Some(supervisor) => {
                let watcher_deadline = deadline;
                let shutdown = tokio::task::spawn_blocking(move || {
                    supervisor.shutdown_bounded(remaining_shutdown_budget(
                        watcher_deadline,
                        tokio::time::Instant::now(),
                    ))
                });
                match tokio::time::timeout_at(deadline, shutdown).await {
                    Ok(Ok(result)) => result.map_err(anyhow::Error::from),
                    Ok(Err(_)) => Err(anyhow::anyhow!("watcher shutdown coordination task failed")),
                    Err(_) => Err(anyhow::anyhow!(
                        "watcher shutdown coordination deadline exceeded"
                    )),
                }
            }
            None => Ok(()),
        };
        combine_runtime_shutdown_results(background_result, lifecycle_result, watcher_result)
    }
}

fn combine_runtime_shutdown_results(
    background_result: anyhow::Result<()>,
    lifecycle_result: anyhow::Result<()>,
    watcher_result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut primary = background_result.err();
    if let Err(lifecycle) = lifecycle_result {
        primary = Some(match primary {
            Some(error) => error.context(format!("lifecycle shutdown also failed: {lifecycle}")),
            None => lifecycle,
        });
    }
    if let Err(watcher) = watcher_result {
        primary = Some(match primary {
            Some(error) => error.context(format!("watcher shutdown also failed: {watcher}")),
            None => watcher,
        });
    }
    primary.map_or(Ok(()), Err)
}

impl ServerTransportRuntime {
    pub(crate) async fn serve<F>(
        &self,
        listener: tokio::net::TcpListener,
        launch: ServerLaunchOptions,
        shutdown: F,
    ) -> Result<(), ServerTransportServeError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.serve_with_shutdown_deadline(
            listener,
            launch,
            shutdown,
            RuntimeShutdownDeadline::default(),
        )
        .await
    }

    async fn serve_with_shutdown_deadline<F>(
        &self,
        listener: tokio::net::TcpListener,
        launch: ServerLaunchOptions,
        shutdown: F,
        shutdown_deadline: RuntimeShutdownDeadline,
    ) -> Result<(), ServerTransportServeError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let bound_port = listener
            .local_addr()
            .map_err(|source| ServerTransportServeError {
                source: source.into(),
                sessions_retired: true,
            })?
            .port();
        let launch = launch.with_port(bound_port);
        let runtime_incarnation = self.app_state.repo_lifecycle_jobs().runtime_incarnation();
        let host_peer_id = self.app_state.identity_key.peer_id().to_string();
        let owner_hint = crate::local_cli_proxy_contract::LocalCliOwnerHint::new(
            bound_port,
            host_peer_id.clone(),
            runtime_incarnation,
        );
        runtime::refresh_host_port_hint(&self.host_dir, &owner_hint).map_err(|source| {
            ServerTransportServeError {
                source,
                sessions_retired: true,
            }
        })?;
        runtime::init_node_role(&launch, self.profile, host_peer_id, runtime_incarnation);
        runtime::update_repo_health(
            self.app_state.repo.as_ref(),
            self.app_state.sync_manager.as_ref(),
        );
        let auth =
            runtime::init_auth_runtime(&launch).map_err(|source| ServerTransportServeError {
                source,
                sessions_retired: true,
            })?;
        let ws_transport_runtime = super::ws::transport::WsTransportRuntime::new();
        let app = runtime::build_runtime_router(
            self.app_state.clone(),
            bound_port,
            auth,
            self.p2p_inbound_token_env.clone(),
            launch.runtime_environment(),
            launch.native_allowed_origins(),
            ws_transport_runtime.clone(),
        )
        .map_err(|source| ServerTransportServeError {
            source,
            sessions_retired: true,
        })?;
        let addr = launch.bind_addr();
        println!("Server running on {}", launch.ws_display_base());
        debug_assert_eq!(listener.local_addr().ok(), Some(addr));
        let shutdown_runtime = ws_transport_runtime.clone();
        let signal_deadline = shutdown_deadline.clone();
        let serve_result = serve_router_until_shutdown_with_deadline(listener, app, async move {
            shutdown.await;
            shutdown_runtime.begin_shutdown();
            signal_deadline.begin(SERVER_RUNTIME_SHUTDOWN_TIMEOUT)
        })
        .await;
        ws_transport_runtime.begin_shutdown();
        let deadline = shutdown_deadline.begin(SERVER_RUNTIME_SHUTDOWN_TIMEOUT);
        let session_result = ws_transport_runtime
            .wait_for_idle(remaining_shutdown_budget(
                deadline,
                tokio::time::Instant::now(),
            ))
            .await;
        match (serve_result, session_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(source), Ok(())) => Err(ServerTransportServeError {
                source,
                sessions_retired: true,
            }),
            (Ok(()), Err(source)) => Err(ServerTransportServeError {
                source,
                sessions_retired: false,
            }),
            (Err(serve_error), Err(session_error)) => Err(ServerTransportServeError {
                source: serve_error.context(format!(
                    "WS transport session shutdown also failed: {session_error}"
                )),
                sessions_retired: false,
            }),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn start_server_with_bound_listener(
    repo: Arc<RepoManager>,
    launch: ServerLaunchOptions,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    p2p: P2pConfig,
    listener: tokio::net::TcpListener,
) -> anyhow::Result<()> {
    start_server_with_bound_listener_until_shutdown(
        repo,
        launch,
        plugins,
        profile,
        sync_mode,
        p2p,
        listener,
        shutdown_signal::production_shutdown_signal(),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_server_with_bound_listener_until_shutdown<F>(
    repo: Arc<RepoManager>,
    launch: ServerLaunchOptions,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    p2p: P2pConfig,
    listener: tokio::net::TcpListener,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let runtime =
        EmbeddedServerRuntime::initialize(repo, &launch, plugins, profile, sync_mode, p2p, true)?;
    let transport = runtime.transport();
    let shutdown_deadline = RuntimeShutdownDeadline::default();
    let serve_result = transport
        .serve_with_shutdown_deadline(listener, launch, shutdown, shutdown_deadline.clone())
        .await;
    let shutdown_result = runtime
        .shutdown_until(shutdown_deadline.begin(SERVER_RUNTIME_SHUTDOWN_TIMEOUT))
        .await;
    match (serve_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error.into_anyhow()),
        (Ok(()), Err(error)) => Err(error),
        (Err(serve_error), Err(shutdown_error)) => Err(serve_error.into_anyhow().context(format!(
            "server runtime shutdown also failed: {shutdown_error}"
        ))),
    }
}

pub(super) fn build_sync_engine(
    peer_id: PeerId,
    repo: Arc<RepoManager>,
    sync_mode: SyncMode,
) -> Arc<RepoScopedSyncEngine> {
    runtime::build_sync_engine(peer_id, repo, sync_mode)
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}

#[cfg(test)]
mod tests;
