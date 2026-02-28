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

use axum::{
    Router,
    routing::{get, post},
};
use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::plugin::runtime::host;
use deve_core::protocol::ServerMessage;
use deve_core::sync::engine::SyncEngine;
use deve_core::tree::TreeManager;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;

#[cfg(feature = "search")]
use deve_core::search::SearchService;

pub mod agent_bridge;
pub mod ai_chat;
pub mod auth;
pub mod channel;
pub mod handlers;
pub mod mcp;
pub mod metrics;
pub mod node_role;
pub mod node_role_http;
pub mod plugin_host;
pub mod prewarm;
mod rate_limit;
pub mod security;
pub mod session;
mod setup;
pub mod source_control_proxy;
pub mod ws;

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
    node_role::set_node_role(node_role::NodeRole {
        role: "main".into(),
        ws_port: port,
        main_port: port,
    });
    ai_chat::init_chat_stream_handler()?;
    metrics::init_start_time();
    let mcp_manager = Arc::new(setup::load_mcp_manager(&vault_path));
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

    // 初始化文件树管理器 (从 Ledger Node 表加载)
    let tree_manager = {
        let mut tm = TreeManager::new();
        if let Ok(nodes) = repo.list_local_nodes(None) {
            tm.init_from_nodes(nodes);
        }
        Arc::new(RwLock::new(tm))
    };

    // SPAWN WATCHER
    setup::spawn_file_watcher(
        repo.clone(),
        sync_manager.clone(),
        vault_path.clone(),
        tree_manager.clone(),
        tx.clone(),
    );

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

    // 启动系统指标广播任务 (每 5 秒)
    metrics::spawn_broadcaster(app_state.clone());

    // --- 认证配置加载 ---
    let auth_config = load_auth_config();
    let auth_config = Arc::new(auth_config);
    let brute_force = Arc::new(auth::brute_force::BruteForceGuard::new());

    // 速率限制: 每 IP 每分钟最多 200 次请求
    let limiter = rate_limit::RateLimiter::new(200, std::time::Duration::from_secs(60));

    // 需要认证的路由 (JWT Cookie 中间件保护)
    let protected = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route(
            "/api/sc/status",
            get(handlers::source_control::http::status),
        )
        .route("/api/sc/diff", get(handlers::source_control::http::diff))
        .route("/api/sc/stage", post(handlers::source_control::http::stage))
        .route(
            "/api/sc/commit",
            post(handlers::source_control::http::commit),
        )
        .route("/api/repo/docs", get(handlers::repo::http::list_docs))
        .route("/api/repo/doc", get(handlers::repo::http::doc_content))
        .route("/api/auth/logout", post(auth::handlers::logout))
        .route("/api/auth/me", get(auth::handlers::me))
        .layer(axum::middleware::from_fn(auth::middleware::auth_middleware));

    // 公开路由 (无需认证)
    let public = Router::new()
        .route("/api/auth/login", post(auth::handlers::login))
        .route("/api/node/role", get(node_role_http::role));

    let app = Router::new()
        .merge(protected)
        .merge(public)
        .with_state(app_state)
        .layer(axum::middleware::from_fn(auth::headers::security_headers))
        .layer(axum::middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(brute_force))
        .layer(axum::Extension(limiter))
        .layer(setup::build_cors_layer(port));

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

/// 加载认证配置: 优先环境变量，回退到 dev 默认
fn load_auth_config() -> deve_core::security::AuthConfig {
    match deve_core::security::AuthConfig::from_env() {
        Ok(cfg) => {
            tracing::info!("Auth config loaded from env (user={})", cfg.username);
            cfg
        }
        Err(_) => {
            tracing::warn!("⚠ Auth: env vars not set, using dev defaults (admin/admin)");
            deve_core::security::AuthConfig::dev_default()
                .expect("Dev auth config should always succeed")
        }
    }
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    plugin_host::start_plugin_host_only(plugins, port).await
}
