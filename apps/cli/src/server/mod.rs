// apps\cli\src\server
//! # WebSocket 服务器模块
//!
//! 本模块实现 Deve-Note 的后端 WebSocket 服务器。
//!
//! ## 组件说明
//!
//! - `AppState`: 共享应用状态（Ledger、同步管理器、广播通道）
//! - `start_server`: 启动 HTTP/WebSocket 服务器的主入口
//! - `ws`: WebSocket 连接处理和消息路由
//! - `handlers`: 客户端消息的业务逻辑
//!
//! 服务器使用 Axum 处理 HTTP/WebSocket，并向所有客户端广播变更。

use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::plugin::runtime::host;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

#[cfg(feature = "search")]
use deve_core::search::SearchService;
use tree_state::RepoTreeRegistry;

pub mod agent_bridge;
pub mod ai_chat;
pub mod auth;
pub mod channel;
#[cfg(test)]
mod docs_create_test;
#[cfg(test)]
mod docs_dir_copy_test;
#[cfg(test)]
mod docs_projection_repair_test;
#[cfg(test)]
mod document_remote_scope_test;
#[cfg(test)]
mod edit_scope_test;
mod error_classify;
pub mod handlers;
#[cfg(test)]
mod key_exchange_test;
#[cfg(test)]
mod listing_scope_cleanup_test;
pub mod mcp;
pub mod metrics;
pub mod node_role;
pub mod node_role_http;
mod notegit;
#[cfg(test)]
mod open_doc_scope_test;
pub mod plugin_host;
pub mod plugin_response;
pub mod prewarm;
mod rate_limit;
mod repo_scope;
#[cfg(test)]
mod repo_scope_local_alias_test;
#[cfg(test)]
mod repo_scope_recovery_support;
#[cfg(test)]
mod repo_scope_recovery_test;
#[cfg(test)]
mod repo_scope_recovery_test_extra;
#[cfg(test)]
mod repo_scope_remote_selector_test;
#[cfg(test)]
mod repo_scope_runtime_selector_test;
#[cfg(test)]
mod repo_scope_test;
mod router;
pub mod security;
pub mod session;
mod setup;
#[cfg(test)]
mod source_control_changes_identity_test;
#[cfg(test)]
mod source_control_commit_diff_test;
#[cfg(test)]
mod source_control_http_test;
#[cfg(test)]
mod source_control_local_commit_scope_test;
#[cfg(test)]
mod source_control_local_scope_test;
pub mod source_control_proxy;
#[cfg(test)]
mod source_control_remote_scope_test;
#[cfg(test)]
mod source_control_remote_selector_test;
#[cfg(test)]
mod source_control_scope_test;
#[cfg(test)]
mod source_control_test_support;
pub mod static_files;
#[cfg(test)]
mod switcher_branch_scope_test;
#[cfg(test)]
mod switcher_branch_scope_test_extra;
#[cfg(test)]
mod switcher_branch_scope_test_fail_closed;
#[cfg(test)]
mod switcher_branch_test;
#[cfg(test)]
mod switcher_current_scope_remote_test;
#[cfg(test)]
mod switcher_current_scope_test;
#[cfg(test)]
mod switcher_exact_selector_test;
#[cfg(test)]
mod switcher_scope_rebind_test;
#[cfg(test)]
mod sync_hello_browser_test;
#[cfg(test)]
mod sync_hello_test;
#[cfg(test)]
mod sync_scope_cleanup_test;
mod tree_state;
pub mod ws;

pub struct AppState {
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub plugins: Vec<Box<dyn PluginRuntime>>,
    pub sync_engine: Arc<RepoScopedSyncEngine>,
    /// Repo-scoped 文件树状态。
    pub tree_manager: Arc<RepoTreeRegistry>,
    #[cfg(feature = "search")]
    pub search_service: Option<SearchService>,
    pub identity_key: Arc<deve_core::security::IdentityKeyPair>,
}

pub async fn start_server(
    repo: Arc<RepoManager>,
    vault_path: std::path::PathBuf,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
) -> anyhow::Result<()> {
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    let _ = host::set_repo_manager(repo.clone());
    node_role::set_node_role(node_role::NodeRole {
        role: "main".into(),
        ws_port: port,
        main_port: port,
    });
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    let host_dir = notegit::prepare(repo.as_ref(), &vault_path)?;
    if let Some(host_root) = host_dir.parent() {
        let _ = std::fs::write(host_root.join("main_port"), port.to_string());
    }
    let auth_config = Arc::new(router::load_auth_config());
    let mcp_manager = Arc::new(setup::load_mcp_manager(repo.ledger_dir()));
    let _ = host::set_mcp_manager(mcp_manager.clone());
    // Create broadcast channel for WS server
    let (tx, _rx) = broadcast::channel(100);

    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(
        repo.clone(),
        vault_path.clone(),
    ));
    let _ = host::set_sync_manager(sync_manager.clone());

    prewarm::spawn_prewarm(repo.clone());

    #[cfg(feature = "search")]
    let search_service = {
        let index_path = host_dir.join("search-index");
        match SearchService::new_on_disk(&index_path) {
            Ok(s) => {
                tracing::info!("Search service initialized at {:?}", index_path);
                Some(s)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize search service: {:?}", e);
                None
            }
        }
    };

    // Load or generate Identity Key
    let key_pair = security::load_or_generate_identity_key(&host_dir)?;
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    // Initialize RepoScopedSyncEngine (Relay Mode -> Auto)
    let sync_engine = Arc::new(RepoScopedSyncEngine::new(
        peer_id.clone(),
        repo.clone(),
        deve_core::config::SyncMode::Auto,
    ));
    let tree_manager = Arc::new(RepoTreeRegistry::new());

    // SPAWN WATCHER
    setup::spawn_file_watcher(sync_manager.clone(), vault_path.clone(), tx.clone());

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

    // 启动系统指标广播任务 (每 5 秒)
    metrics::spawn_broadcaster(app_state.clone());

    let app = router::build_app(app_state, port, auth_config);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on ws://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}
