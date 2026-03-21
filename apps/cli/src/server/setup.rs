// apps/cli/src/server/setup.rs
//! 服务器启动辅助: CORS 配置、MCP 加载、文件监视器

use anyhow::{Context, Result, anyhow};
use deve_core::mcp::{McpManager, McpServerConfig};
use deve_core::protocol::ServerMessage;
#[cfg(feature = "search")]
use deve_core::search::SearchService;

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

pub(super) fn write_main_port_hint(host_dir: &std::path::Path, port: u16) -> Result<()> {
    let Some(host_root) = host_dir.parent() else {
        return Err(anyhow!(
            "Host directory has no parent while writing main port hint"
        ));
    };
    let hint_path = host_root.join("main_port");
    std::fs::write(&hint_path, port.to_string())
        .with_context(|| format!("Failed to write main port hint: {:?}", hint_path))
}

pub(super) fn load_mcp_manager(ledger_dir: &std::path::Path) -> McpManager {
    let mut manager = McpManager::new();
    let cfg_path = deve_core::utils::notegit::host_mcp_config_path(ledger_dir);
    match cfg_path.try_exists() {
        Ok(true) => {}
        Ok(false) => return manager,
        Err(err) => {
            tracing::warn!("Failed to stat MCP config {:?}: {}", cfg_path, err);
            return manager;
        }
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

#[cfg(feature = "search")]
pub(super) fn load_search_service(host_dir: &std::path::Path) -> Result<SearchService> {
    let index_path = host_dir.join("search-index");
    SearchService::new_on_disk(&index_path)
        .with_context(|| format!("Failed to initialize search service at {:?}", index_path))
}

/// 启动文件系统监视器 (blocking task)
pub(super) fn spawn_file_watcher(
    sync_manager: Arc<deve_core::sync::SyncManager>,
    vault_path: std::path::PathBuf,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<()> {
    validate_file_watcher_startup(&vault_path)?;
    tokio::task::spawn_blocking(move || {
        use deve_core::watcher::FsEventType;

        let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path).with_callback(
            move |event| match event {
                FsEventType::DocChange(msgs) => {
                    for msg in msgs {
                        match msg {
                            ServerMessage::FsChangeDetected { .. } => {
                                let _ = tx.send(msg);
                            }
                            other => {
                                tracing::error!(
                                    "Watcher emitted unexpected doc-change message: {:?}",
                                    other
                                );
                            }
                        }
                    }
                }
                FsEventType::DirChange { repo_id, path } => {
                    tracing::info!("DirChange detected: repo-scoped refresh ({})", path);
                    let _ = tx.send(ServerMessage::FsChangeDetected {
                        repo_id: Some(repo_id),
                        branch: None,
                        scope_nonce: None,
                        path,
                        change_type: "dir_changed".to_string(),
                        has_conflict: false,
                    });
                }
            },
        );

        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });
    Ok(())
}

fn validate_file_watcher_startup(vault_path: &std::path::Path) -> Result<()> {
    deve_core::watcher::validate_watch_root(vault_path)
        .with_context(|| format!("Watcher startup preflight failed for {:?}", vault_path))
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "search")]
    use super::load_search_service;
    use super::{validate_file_watcher_startup, write_main_port_hint};

    #[test]
    fn write_main_port_hint_fails_closed_when_parent_is_not_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bad_root = dir.path().join("host-root");
        std::fs::write(&bad_root, "not-a-dir").expect("bad root file");

        let err = write_main_port_hint(&bad_root.join(".host"), 3001)
            .expect_err("invalid host root must fail closed");

        assert!(
            err.to_string().contains("Failed to write main port hint")
                || err.to_string().contains("Not a directory")
        );
    }

    #[test]
    fn validate_file_watcher_startup_fails_closed_on_missing_vault_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = validate_file_watcher_startup(&dir.path().join("missing-vault"))
            .expect_err("missing vault root must fail closed");

        assert!(
            err.to_string().contains("Watcher startup preflight failed")
                || err.to_string().contains("canonicalize")
                || err.to_string().contains("No such file")
        );
    }

    #[cfg(feature = "search")]
    #[test]
    fn load_search_service_fails_closed_when_index_path_is_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let host_dir = dir.path().join(".host");
        std::fs::create_dir_all(&host_dir).expect("host dir");
        std::fs::write(host_dir.join("search-index"), "not-a-dir").expect("index file");

        let err = match load_search_service(&host_dir) {
            Ok(_) => panic!("broken search index path must fail"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("Failed to initialize search service")
        );
    }
}
