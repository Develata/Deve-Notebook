//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! HTTP/WebSocket server boot sequence.

use super::{
    AppState, ai_chat, metrics, node_role, notegit, plugin_host, prewarm, router, security, setup,
    static_files, tree_state::RepoTreeRegistry,
};
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::{PluginRuntime, host};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn start_server(
    repo: Arc<RepoManager>,
    vault_path: std::path::PathBuf,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg_attr(not(feature = "search"), allow(unused_variables))]
    profile: deve_core::config::AppProfile,
) -> anyhow::Result<()> {
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    host::set_repo_manager(repo.clone())?;
    node_role::set_node_role(node_role::NodeRole {
        role: "main".into(),
        ws_port: port,
        main_port: port,
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
    host::set_sync_manager(sync_manager.clone())?;

    prewarm::spawn_prewarm(repo.clone());

    #[cfg(feature = "search")]
    let search_service = if profile == deve_core::config::AppProfile::LowSpec {
        tracing::info!("LowSpec profile: search service disabled");
        None
    } else {
        Some(setup::load_search_service(&host_dir)?)
    };

    let key_pair = security::load_or_generate_identity_key(&host_dir)?;
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    let sync_engine = Arc::new(RepoScopedSyncEngine::new(
        peer_id.clone(),
        repo.clone(),
        deve_core::config::SyncMode::Auto,
    ));
    let tree_manager = Arc::new(RepoTreeRegistry::new());
    let watcher_ids = setup::start_file_watchers(repo.as_ref(), sync_manager.clone(), tx.clone())?;

    let app_state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager,
        tx,
        plugins,
        sync_engine,
        tree_manager,
        #[cfg(feature = "search")]
        search_service,
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

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}
