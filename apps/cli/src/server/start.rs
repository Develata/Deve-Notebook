//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! HTTP/WebSocket server boot sequence.

use super::{
    AppState, ai_chat, metrics, node_role, notegit, plugin_host, prewarm, router, security, setup,
    static_files, tree_state::RepoTreeRegistry,
};
use crate::server::launch::ServerLaunchOptions;
use deve_core::config::{AppProfile, P2pConfig, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::{PluginRuntime, host};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::{models::PeerId, sync::repo_scoped};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn start_server(
    repo: Arc<RepoManager>,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
    p2p: P2pConfig,
) -> anyhow::Result<()> {
    start_server_with_options(
        repo,
        ServerLaunchOptions::release(port),
        plugins,
        profile,
        sync_mode,
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
    p2p: P2pConfig,
) -> anyhow::Result<()> {
    let port = launch.port();
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    let source_control_api: Arc<dyn deve_core::source_control::SourceControlApi> = repo.clone();
    host::set_source_control_api(source_control_api)?;
    host::set_repo_manager(repo.clone())?;
    node_role::set_node_role(node_role::NodeRole {
        role: launch.node_role_label().into(),
        ws_port: port,
        main_port: port,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: profile_label(profile).into(),
        delivery: static_files::delivery_shape().into(),
        environment: node_role::runtime_environment(),
        repo_health: node_role::RepoHealthSummary::unknown(),
        native_service: launch.native_service_summary(),
    });
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    let host_dir = notegit::prepare(repo.as_ref())?;
    setup::write_main_port_hint(&host_dir, port)?;
    let auth_config = Arc::new(router::load_auth_config());
    let native_session_bridge =
        super::auth::handlers::NativeSessionBridge::from_env(launch.is_native_loopback())
            .map(|bridge| bridge.map(Arc::new))?;
    let (tx, _rx) = broadcast::channel(100);

    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    sync_manager.scan()?;
    node_role::update_repo_health(repo_health_summary(repo.as_ref(), sync_manager.as_ref()));
    host::set_sync_manager(sync_manager.clone())?;

    prewarm::spawn_prewarm(repo.clone());

    #[cfg(feature = "search")]
    let search_available = if profile == deve_core::config::AppProfile::LowSpec {
        tracing::info!("LowSpec profile: search service disabled");
        false
    } else {
        tracing::info!("Search baseline scan enabled");
        true
    };

    let key_pair = security::load_or_generate_identity_key(&host_dir)?;
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    let sync_engine = build_sync_engine(peer_id.clone(), repo.clone(), sync_mode);
    let tree_manager = Arc::new(RepoTreeRegistry::new());
    let watcher_ids = setup::start_file_watchers(sync_manager.clone(), tx.clone())?;

    let app_state = Arc::new(AppState {
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

    metrics::spawn_broadcaster(app_state.clone());
    super::p2p::spawn_mesh_connectors(p2p, app_state.clone());

    static_files::validate_static_dir_override()?;
    let app = match native_session_bridge {
        Some(bridge) => {
            router::build_app_with_native_session(app_state, port, auth_config, Some(bridge))?
        }
        None => router::build_app(app_state, port, auth_config)?,
    };
    let addr = launch.bind_addr();
    println!("Server running on {}", launch.ws_display_base());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;
    for repo_id in watcher_ids {
        let _ = deve_core::sync::watcher::stop_repo_watcher(repo_id);
    }
    result?;
    Ok(())
}

pub(super) fn build_sync_engine(
    peer_id: PeerId,
    repo: Arc<RepoManager>,
    sync_mode: SyncMode,
) -> Arc<repo_scoped::RepoScopedSyncEngine> {
    Arc::new(RepoScopedSyncEngine::new(peer_id, repo, sync_mode))
}

fn profile_label(profile: AppProfile) -> &'static str {
    match profile {
        AppProfile::Standard => "standard",
        AppProfile::LowSpec => "low-spec",
    }
}

fn repo_health_summary(
    repo: &RepoManager,
    sync_manager: &deve_core::sync::SyncManager,
) -> node_role::RepoHealthSummary {
    let local_total = match repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos.len(),
        Err(err) => {
            tracing::warn!("Failed to list repos for node role health: {}", err);
            return node_role::RepoHealthSummary::unknown();
        }
    };
    match sync_manager.degraded_local_repo_names_for_execution() {
        Ok(degraded) => {
            node_role::RepoHealthSummary::from_degraded_count(local_total, degraded.len())
        }
        Err(err) => {
            tracing::warn!("Failed to summarize repo health for node role: {}", err);
            node_role::RepoHealthSummary::unknown()
        }
    }
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}

#[cfg(test)]
mod tests;
