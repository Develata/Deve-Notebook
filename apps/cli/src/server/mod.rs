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

use axum::{Router, routing::{get, post}};
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::plugin::runtime::host;
use deve_core::protocol::ServerMessage;
use deve_core::sync::engine::SyncEngine;
use deve_core::tree::TreeManager;
use deve_core::mcp::{McpManager, McpServerConfig};

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

#[cfg(feature = "search")]
use deve_core::search::SearchService;

pub mod channel;
pub mod ai_chat;
pub mod mcp;
pub mod handlers;
pub mod session;
pub mod ws;
pub mod source_control_proxy;
pub mod plugin_host;
pub mod node_role;
pub mod node_role_http;
pub mod security;
pub mod prewarm;

#[allow(dead_code)] // repo_key: 为未来加密功能预留
pub struct AppState {
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub vault_path: std::path::PathBuf,
    pub plugins: Vec<Box<dyn PluginRuntime>>,
    pub sync_engine: Arc<RwLock<SyncEngine>>,
    /// 文件树管理器 (增量更新)
    pub tree_manager: Arc<RwLock<TreeManager>>,
    #[cfg(feature = "search")]
    pub search_service: Option<SearchService>,
    pub identity_key: Arc<deve_core::security::IdentityKeyPair>,
    pub repo_key: Option<deve_core::security::RepoKey>,
}

pub async fn start_server(
    repo: Arc<RepoManager>,
    vault_path: std::path::PathBuf,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
) -> anyhow::Result<()> {
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    node_role::set_node_role(node_role::NodeRole { role: "main".into(), ws_port: port, main_port: port });
    ai_chat::init_chat_stream_handler()?;
    let mcp_manager = Arc::new(load_mcp_manager(&vault_path));
    let _ = host::set_mcp_manager(mcp_manager.clone());
    // Create broadcast channel for WS server
    let (tx, _rx) = broadcast::channel(100);

    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(
        repo.clone(),
        vault_path.clone(),
    ));

    prewarm::spawn_prewarm(repo.clone());

    #[cfg(feature = "search")]
    let search_service = {
        let index_path = vault_path.join(".deve_search_index");
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
    let deve_dir = vault_path.join(".deve");
    std::fs::create_dir_all(&deve_dir)?;

    let key_pair = security::load_or_generate_identity_key(&deve_dir)?;
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    // Load or generate Repo Key (Shared Secret)
    let repo_key = security::load_or_generate_repo_key(&deve_dir)?;

    // Initialize SyncEngine (Relay Mode -> Auto)
    let sync_engine = Arc::new(RwLock::new(SyncEngine::new(
        peer_id.clone(),
        repo.clone(),
        deve_core::config::SyncMode::Auto,
        repo_key.clone(),
    )));

    // 初始化文件树管理器 (从 Ledger 加载文档列表)
    let tree_manager = {
        let mut tm = TreeManager::new();
        let repo_id = repo
            .get_repo_info()
            .ok()
            .flatten()
            .map(|info| info.uuid)
            .unwrap_or_else(uuid::Uuid::nil);
        if let Ok(docs) = repo.list_docs(&deve_core::models::RepoType::Local(repo_id)) {
            tm.init_from_docs(docs);
        }
        Arc::new(RwLock::new(tm))
    };

    // SPAWN WATCHER
    let tx_for_watcher = tx.clone();
    let sm_for_watcher = sync_manager.clone();
    let vp_for_watcher = vault_path.clone();
    let tm_for_watcher = tree_manager.clone();

    tokio::task::spawn_blocking(move || {
        let watcher = deve_core::watcher::Watcher::new(sm_for_watcher, vp_for_watcher)
            .with_callback(move |msgs| {
                for msg in msgs {
                    if let ServerMessage::DocList { docs } = &msg {
                        // watcher 触发的 DocList 代表文件结构变动或需要刷新树
                        match tm_for_watcher.write() {
                            Ok(mut tm) => {
                                tm.init_from_docs(docs.clone());
                                let delta = tm.build_init_delta();
                                let _ = tx_for_watcher.send(ServerMessage::TreeUpdate(delta));
                            }
                            Err(e) => {
                                tracing::error!("TreeManager lock failed: {:?}", e);
                            }
                        }
                    }
                    let _ = tx_for_watcher.send(msg);
                }
            });

        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });

    let app_state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager,
        tx,
        vault_path,
        plugins,
        sync_engine,
        tree_manager,
        #[cfg(feature = "search")]
        search_service,
        identity_key: key_pair,
        repo_key,
    });

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/api/node/role", get(node_role_http::role))
        .route("/api/sc/status", get(handlers::source_control::http::status))
        .route("/api/sc/diff", get(handlers::source_control::http::diff))
        .route("/api/sc/stage", post(handlers::source_control::http::stage))
        .route("/api/sc/commit", post(handlers::source_control::http::commit))
        .route("/api/repo/docs", get(handlers::repo::http::list_docs))
        .route("/api/repo/doc", get(handlers::repo::http::doc_content))
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on ws://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}

fn load_mcp_manager(vault_path: &std::path::Path) -> McpManager {
    let mut manager = McpManager::new();
    let cfg_path = vault_path.join(".deve").join("mcp.json");
    if !cfg_path.exists() {
        return manager;
    }

    let content = match std::fs::read_to_string(&cfg_path) {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("Failed to read MCP config: {:?}", err);
            return manager;
        }
    };

    let configs: Vec<McpServerConfig> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Invalid MCP config: {:?}", err);
            return manager;
        }
    };

    mcp::register_mcp_servers(&mut manager, configs);

    manager
}
