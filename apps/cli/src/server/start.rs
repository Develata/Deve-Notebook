//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! HTTP/WebSocket server boot sequence.

use super::{plugin_host, runtime};
use crate::server::launch::ServerLaunchOptions;
use deve_core::config::{AppProfile, GitBridgeMode, P2pConfig, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;

pub async fn start_server(
    repo: Arc<RepoManager>,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    git_bridge: GitBridgeMode,
    p2p: P2pConfig,
) -> anyhow::Result<()> {
    start_server_with_options(
        repo,
        ServerLaunchOptions::release(port),
        plugins,
        profile,
        sync_mode,
        git_bridge,
        p2p,
    )
    .await
}

pub async fn start_server_with_options(
    repo: Arc<RepoManager>,
    launch: ServerLaunchOptions,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    git_bridge: GitBridgeMode,
    p2p: P2pConfig,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(launch.bind_addr()).await?;
    start_server_with_bound_listener(
        repo, launch, plugins, profile, sync_mode, git_bridge, p2p, listener,
    )
    .await
}

pub async fn start_server_with_bound_listener(
    repo: Arc<RepoManager>,
    launch: ServerLaunchOptions,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    git_bridge: GitBridgeMode,
    p2p: P2pConfig,
    listener: tokio::net::TcpListener,
) -> anyhow::Result<()> {
    let bound_port = listener.local_addr()?.port();
    let launch = launch.with_port(bound_port);
    let port = launch.port();
    runtime::install_repo_host_apis(&repo, git_bridge)?;
    runtime::init_node_role(&launch, profile, git_bridge);
    runtime::init_observability_runtime()?;
    let host_dir = runtime::prepare_host_layout(repo.as_ref(), port)?;
    let auth = runtime::init_auth_runtime(&launch)?;
    let tx = runtime::new_server_broadcast_channel();

    let sync_manager = runtime::init_sync_manager(repo.clone())?;
    runtime::update_repo_health(repo.as_ref(), sync_manager.as_ref());
    runtime::install_sync_host_api(sync_manager.clone())?;

    runtime::spawn_prewarm(repo.clone());

    #[cfg(feature = "search")]
    let search_available = runtime::search_available(profile);

    let key_pair = runtime::load_identity_key(&host_dir)?;
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    let sync_engine = build_sync_engine(peer_id.clone(), repo.clone(), sync_mode);
    let tree_manager = runtime::build_tree_registry();
    let _watchers = runtime::start_file_watchers(sync_manager.clone(), tx.clone())?;

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
        git_bridge,
    });

    let p2p_inbound_token_env = p2p.inbound_token_env.clone();
    runtime::spawn_background_runtime_tasks(p2p, app_state.clone());

    let app = runtime::build_runtime_router(
        app_state,
        port,
        auth,
        p2p_inbound_token_env,
        launch.native_allowed_origins(),
    )?;
    let addr = launch.bind_addr();
    println!("Server running on {}", launch.ws_display_base());
    debug_assert_eq!(listener.local_addr()?, addr);
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
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
