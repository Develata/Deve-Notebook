// apps/cli/src/server/setup.rs
//! 服务器启动辅助: CORS 配置、MCP 加载、文件监视器

use deve_core::mcp::{McpManager, McpServerConfig};
use deve_core::protocol::ServerMessage;

use axum::http::{Method, header};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// 按环境变量构建 CORS 层；默认不信任任何跨站来源，禁止生产硬编码 localhost。
pub(super) fn build_cors_layer(_port: u16) -> CorsLayer {
    let origins = allowed_origins_from_env();
    if is_development() && !origins.is_empty() {
        tracing::warn!("WARNING: CORS development allow list active");
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ORIGIN,
        ])
        .allow_credentials(true)
}

/// 从 `ALLOWED_ORIGINS` 解析允许的跨站来源列表，使用逗号分隔。
fn allowed_origins_from_env() -> Vec<axum::http::HeaderValue> {
    std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty() && *origin != "*")
        .filter_map(|origin| match origin.parse() {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!(origin, ?err, "Ignoring invalid CORS origin");
                None
            }
        })
        .collect()
}

/// 判断当前是否为开发模式；仅显式 `DEVE_ENV=development` 视为开发环境。
fn is_development() -> bool {
    matches!(std::env::var("DEVE_ENV"), Ok(value) if value.eq_ignore_ascii_case("development"))
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
    sync_manager: Arc<deve_core::sync::SyncManager>,
    vault_path: std::path::PathBuf,
    tx: broadcast::Sender<ServerMessage>,
) {
    let local_repo_id = sync_manager.local_repo_id();
    tokio::task::spawn_blocking(move || {
        use deve_core::watcher::FsEventType;

        let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path).with_callback(
            move |event| match event {
                FsEventType::DocChange(msgs) => {
                    for msg in msgs {
                        if matches!(msg, ServerMessage::FsChangeDetected { .. }) {
                            let _ = tx.send(msg);
                        }
                    }
                }
                FsEventType::DirChange => {
                    tracing::warn!("DirChange detected: triggering session-scoped refresh");
                    let _ = tx.send(ServerMessage::FsChangeDetected {
                        repo_id: local_repo_id,
                        path: String::new(),
                        change_type: "dir_changed".to_string(),
                        has_conflict: false,
                    });
                }
                FsEventType::FsPendingChange { path, change_type } => {
                    tracing::info!("FsPendingChange: {} ({})", path, change_type);
                    let _ = tx.send(ServerMessage::FsChangeDetected {
                        repo_id: local_repo_id,
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
