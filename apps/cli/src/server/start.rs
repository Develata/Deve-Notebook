//! plan_ref:
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
    background_tasks: Option<runtime::BackgroundRuntimeTasks>,
    watcher_guard: Option<runtime::watcher_runtime::FileWatcherRuntimeGuard>,
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
        runtime::init_observability_runtime()?;
        let host_dir = runtime::prepare_host_layout(repo.as_ref(), launch.port())?;
        let tx = runtime::new_server_broadcast_channel();

        let sync_manager = runtime::init_sync_manager(repo.clone())?;
        runtime::install_sync_host_api(sync_manager.clone())?;

        #[cfg(feature = "search")]
        let search_available = runtime::search_available(profile);

        let key_pair = runtime::load_identity_key(&host_dir)?;
        let peer_id = key_pair.peer_id();
        tracing::info!("Server PeerID: {}", peer_id);

        let sync_engine = build_sync_engine(peer_id, repo.clone(), sync_mode);
        let tree_manager = runtime::build_tree_registry();
        let watcher_guard = runtime::start_file_watchers(sync_manager.clone(), tx.clone())?;
        let app_state = runtime::build_app_state(runtime::AppStateParts {
            repo: repo.clone(),
            sync_manager,
            tx,
            plugins,
            sync_engine,
            tree_manager,
            #[cfg(feature = "search")]
            search_available,
            identity_key: key_pair,
        });
        let p2p_inbound_token_env = p2p.inbound_token_env.clone();
        let background_tasks =
            runtime::spawn_background_runtime_tasks(p2p, app_state.clone(), repo, prewarm_enabled);

        Ok(Self {
            transport: ServerTransportRuntime {
                app_state,
                profile,
                host_dir: Arc::new(host_dir),
                p2p_inbound_token_env,
            },
            background_tasks: Some(background_tasks),
            watcher_guard: Some(watcher_guard),
        })
    }

    pub(crate) fn transport(&self) -> ServerTransportRuntime {
        self.transport.clone()
    }

    pub(crate) async fn shutdown(mut self, timeout: Duration) -> anyhow::Result<()> {
        let background_result = match self.background_tasks.take() {
            Some(tasks) => tasks.shutdown(timeout).await,
            None => Ok(()),
        };
        drop(self.watcher_guard.take());
        background_result
    }
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
        let bound_port = listener
            .local_addr()
            .map_err(|source| ServerTransportServeError {
                source: source.into(),
                sessions_retired: true,
            })?
            .port();
        let launch = launch.with_port(bound_port);
        runtime::refresh_host_port_hint(&self.host_dir, bound_port).map_err(|source| {
            ServerTransportServeError {
                source,
                sessions_retired: true,
            }
        })?;
        runtime::init_node_role(&launch, self.profile);
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
        let serve_result = serve_router_until_shutdown(listener, app, async move {
            shutdown.await;
            shutdown_runtime.begin_shutdown();
        })
        .await;
        ws_transport_runtime.begin_shutdown();
        let session_result = ws_transport_runtime
            .wait_for_idle(SERVER_RUNTIME_SHUTDOWN_TIMEOUT)
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
        std::future::pending(),
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
    let serve_result = transport.serve(listener, launch, shutdown).await;
    let shutdown_result = runtime.shutdown(SERVER_RUNTIME_SHUTDOWN_TIMEOUT).await;
    match (serve_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error.into_anyhow()),
        (Ok(()), Err(error)) => Err(error),
        (Err(serve_error), Err(shutdown_error)) => Err(serve_error.into_anyhow().context(format!(
            "server runtime shutdown also failed: {shutdown_error}"
        ))),
    }
}

async fn serve_router_until_shutdown<F>(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await;
    result?;
    Ok(())
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
