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

use axum::{routing::get, Router};
use std::sync::Arc;
use deve_core::ledger::RepoManager;
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use std::net::SocketAddr;
use deve_core::plugin::runtime::PluginRuntime;

#[cfg(feature = "search")]
use deve_core::search::SearchService;

pub mod ws;
pub mod handlers;

pub struct AppState {
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub vault_path: std::path::PathBuf,
    pub plugins: Vec<Box<dyn PluginRuntime>>,
    #[cfg(feature = "search")]
    pub search_service: Option<SearchService>,
}

pub async fn start_server(
    repo: Arc<RepoManager>,
    vault_path: std::path::PathBuf,
    port: u16,
    plugins: Vec<Box<dyn PluginRuntime>>,
) -> anyhow::Result<()> {
    // Create broadcast channel for WS server
    let (tx, _rx) = broadcast::channel(100);
    
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault_path.clone()));

    // SPAWN WATCHER
    let tx_for_watcher = tx.clone();
    let sm_for_watcher = sync_manager.clone();
    let vp_for_watcher = vault_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let watcher = deve_core::watcher::Watcher::new(sm_for_watcher, vp_for_watcher)
            .with_callback(move |msgs| {
                for msg in msgs {
                    let _ = tx_for_watcher.send(msg);
                }
            });
            
        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });

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

    let app_state = Arc::new(AppState { 
        repo: repo.clone(),
        sync_manager,
        tx,
        vault_path,
        plugins,
        #[cfg(feature = "search")]
        search_service,
    });

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
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
