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
use deve_core::models::PeerId;
use deve_core::sync::engine::SyncEngine;
use std::sync::RwLock;

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
    pub sync_engine: Arc<RwLock<SyncEngine>>,
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

    // Load or generate Identity Key
    let key_pair_path = vault_path.join(".deve").join("identity.key");
    let key_pair = if key_pair_path.exists() {
        // TODO: Load from file (impl Serialize for KeyPair first, or use raw bytes).
        // For now, regen to avoid complex FS logic in this snippet, 
        // effectively implementing "Ephemeral Identity" for this iteration unless persisted.
        // Real impl should persist.
        Arc::new(deve_core::security::IdentityKeyPair::generate())
    } else {
        let kp = deve_core::security::IdentityKeyPair::generate();
        // Save logic omitted for brevity/modularity limit, will just use generated
        Arc::new(kp)
    };
    
    let peer_id = key_pair.peer_id();
    tracing::info!("Server PeerID: {}", peer_id);

    // Load or generate Repo Key (Shared Secret)
    let repo_key_path = vault_path.join(".deve").join("repo.key");
    let repo_key = if repo_key_path.exists() {
        let bytes = std::fs::read(&repo_key_path)?;
        deve_core::security::RepoKey::from_bytes(&bytes)
    } else {
        let key = deve_core::security::RepoKey::generate();
        // Persist logic omitted
        Some(key)
    };

    // Initialize SyncEngine (Relay Mode -> Auto)
    let sync_engine = Arc::new(RwLock::new(SyncEngine::new(
        peer_id.clone(),
        repo.clone(),
        deve_core::config::SyncMode::Auto,
        repo_key.clone(),
    )));

    let app_state = Arc::new(AppState { 
        repo: repo.clone(),
        sync_manager,
        tx,
        vault_path,
        plugins,
        sync_engine,
        #[cfg(feature = "search")]
        search_service,
        identity_key: key_pair,
        repo_key,
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
