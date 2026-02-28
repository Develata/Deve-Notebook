// apps/cli/src/server/setup.rs
//! 服务器启动辅助: CORS 配置、MCP 加载、文件监视器

use deve_core::ledger::RepoManager;
use deve_core::mcp::{McpManager, McpServerConfig};
use deve_core::protocol::ServerMessage;
use deve_core::tree::TreeManager;

use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

/// 构建 CORS 层 — 仅允许 localhost 来源
pub(super) fn build_cors_layer(port: u16) -> CorsLayer {
    use tower_http::cors::AllowOrigin;
    let origins: Vec<axum::http::HeaderValue> = (port..=port + 4)
        .flat_map(|p| {
            [
                format!("http://localhost:{}", p),
                format!("http://127.0.0.1:{}", p),
            ]
        })
        .filter_map(|s| s.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}

pub(super) fn load_mcp_manager(vault_path: &std::path::Path) -> McpManager {
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

    super::mcp::register_mcp_servers(&mut manager, configs);
    manager
}

/// 启动文件系统监视器 (blocking task)
pub(super) fn spawn_file_watcher(
    repo: Arc<RepoManager>,
    sync_manager: Arc<deve_core::sync::SyncManager>,
    vault_path: std::path::PathBuf,
    tree_manager: Arc<RwLock<TreeManager>>,
    tx: broadcast::Sender<ServerMessage>,
) {
    tokio::task::spawn_blocking(move || {
        use deve_core::watcher::FsEventType;

        let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path)
            .with_callback(move |event| match event {
                FsEventType::DocChange(msgs) => {
                    for msg in msgs {
                        if let Ok(nodes) = repo.list_local_nodes(None)
                            && let Ok(mut tm) = tree_manager.write()
                        {
                            tm.init_from_nodes(nodes);
                            let delta = tm.build_init_delta();
                            let _ = tx.send(ServerMessage::TreeUpdate(delta));
                        }
                        let _ = tx.send(msg);
                    }
                }
                FsEventType::DirChange => {
                    tracing::warn!("DirChange detected: ignore without Node update");
                }
            });

        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });
}
