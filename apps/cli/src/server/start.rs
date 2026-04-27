//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! HTTP/WebSocket server boot sequence.

use super::{
    AppState, ai_chat, metrics, node_role, notegit, plugin_host, prewarm, router, security, setup,
    static_files, tree_state::RepoTreeRegistry,
};
use deve_core::config::{AppProfile, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::{PluginRuntime, host};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::{models::PeerId, sync::repo_scoped};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn start_server(
    repo: Arc<RepoManager>,
    vault_path: std::path::PathBuf,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))] profile: AppProfile,
    sync_mode: SyncMode,
) -> anyhow::Result<()> {
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    host::set_repo_manager(repo.clone())?;
    node_role::set_node_role(node_role::NodeRole {
        role: "main".into(),
        ws_port: port,
        main_port: port,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: profile_label(profile).into(),
        delivery: static_files::delivery_shape().into(),
        environment: node_role::runtime_environment(),
    });
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    let host_dir = notegit::prepare(repo.as_ref(), &vault_path)?;
    setup::write_main_port_hint(&host_dir, port)?;
    let auth_config = Arc::new(router::load_auth_config());
    let mcp_manager = Arc::new(setup::load_mcp_manager(repo.ledger_dir())?);
    host::set_mcp_manager(mcp_manager.clone())?;
    let (tx, _rx) = broadcast::channel(100);

    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(
        repo.clone(),
        vault_path.clone(),
    )?);
    sync_manager.scan()?;
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

    static_files::validate_static_dir_override()?;
    let app = router::build_app(app_state, port, auth_config)?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on ws://{}", addr);

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

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}

#[cfg(test)]
#[path = "start_test.rs"]
mod tests;
