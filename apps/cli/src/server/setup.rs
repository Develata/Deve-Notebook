// apps/cli/src/server/setup.rs
//! 服务器启动辅助: CORS 配置、MCP 加载、文件监视器

use deve_core::ledger::RepoManager;
use deve_core::mcp::{McpManager, McpServerConfig};
use deve_core::protocol::ServerMessage;
use deve_core::tree::TreeManager;

use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// 构建 CORS 层。
///
/// 安全策略：生产环境默认拒绝一切跨域请求，必须显式设置 `ALLOWED_ORIGINS`。
/// 仅在 `DEVE_ENV=development` 且未提供 `ALLOWED_ORIGINS` 时，才回退为本地前端来源。
pub(super) fn build_cors_layer(_port: u16) -> CorsLayer {
    let env = std::env::var("DEVE_ENV").unwrap_or_else(|_| "production".to_string());
    let allowed_origins = std::env::var("ALLOWED_ORIGINS").ok();
    let origins = match (env.as_str(), allowed_origins.as_deref()) {
        (_, Some(origins)) => origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        ("development", None) => {
            tracing::warn!("Development mode: allowing localhost CORS origins");
            vec![
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ]
        }
        _ => panic!("Production mode requires ALLOWED_ORIGINS"),
    };
    if origins.is_empty() {
        panic!("ALLOWED_ORIGINS must contain at least one origin");
    }
    if origins.iter().any(|origin| origin == "*") {
        panic!("ALLOWED_ORIGINS must not contain wildcard '*'");
    }
    let origins = origins
        .into_iter()
        .map(|origin| origin.parse())
        .collect::<Result<Vec<axum::http::HeaderValue>, _>>()
        .unwrap_or_else(|err| panic!("Invalid CORS origin: {err}"));
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

        let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path).with_callback(
            move |event| match event {
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
                FsEventType::FsPendingChange { path, change_type } => {
                    tracing::info!("FsPendingChange: {} ({})", path, change_type);
                    let _ = tx.send(ServerMessage::FsChangeDetected {
                        path,
                        change_type,
                        has_conflict: false,
                    });
                }
            },
        );

        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });
}
