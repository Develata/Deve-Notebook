// apps/cli/src/server/setup.rs
//! plan_ref:
//!   - 04_storage#watcher-contract
//!   - 09_auth#cors
//!   - 09_auth#security-headers
//!   - 15_release#runtime-observability
//!
//! 服务器启动辅助: CORS 配置、文件监视器

use anyhow::{Context, Result, anyhow};
use deve_core::protocol::ServerMessage;

use axum::http::{Method, header};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// 按环境变量构建 CORS 层；默认不信任任何跨站来源，禁止生产硬编码 localhost。
pub(super) fn build_cors_layer(_port: u16) -> Result<CorsLayer> {
    let origins = allowed_origins_from_env()?;
    if is_development() && !origins.is_empty() {
        tracing::warn!(
            "WARNING: development-only CORS allow list active; never use this as production origin policy"
        );
    }

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ORIGIN,
        ])
        .allow_credentials(true))
}

/// 从 `ALLOWED_ORIGINS` 解析允许的跨站来源列表，使用逗号分隔。
fn allowed_origins_from_env() -> Result<Vec<axum::http::HeaderValue>> {
    let Ok(origins) = std::env::var("ALLOWED_ORIGINS") else {
        return Ok(Vec::new());
    };

    origins
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            if origin == "*" {
                return Err(anyhow!(
                    "Wildcard CORS origin is forbidden; set explicit ALLOWED_ORIGINS"
                ));
            }
            let uri: axum::http::Uri = origin
                .parse()
                .with_context(|| format!("Invalid CORS origin {}", origin))?;
            if uri.scheme().is_none() || uri.authority().is_none() {
                return Err(anyhow!("Invalid CORS origin {}", origin));
            }
            origin
                .parse()
                .with_context(|| format!("Invalid CORS origin {}", origin))
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

/// 启动每个本地 repo 的 watcher。
pub(super) fn start_file_watchers(
    sync_manager: Arc<deve_core::sync::SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<Vec<deve_core::models::RepoId>> {
    let mut ids = Vec::new();
    for repo_name in sync_manager.healthy_local_repo_names_for_execution()? {
        let tx_clone = tx.clone();
        let callback = Arc::new(move |msg: ServerMessage| {
            if let ServerMessage::FsChangeDetected { .. } = &msg {
                let _ = tx_clone.send(msg);
            }
        });
        ids.push(deve_core::sync::watcher::start_repo_watcher(
            sync_manager.clone(),
            &repo_name,
            None,
            Some(callback),
        )?);
    }
    Ok(ids)
}

#[cfg(test)]
fn validate_file_watcher_startup(vault_path: &std::path::Path) -> Result<()> {
    deve_core::watcher::validate_watch_root(vault_path)
        .with_context(|| format!("Watcher startup preflight failed for {:?}", vault_path))
}

#[cfg(test)]
mod tests {
    use super::{allowed_origins_from_env, validate_file_watcher_startup, write_main_port_hint};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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

    #[test]
    fn allowed_origins_from_env_fails_closed_on_invalid_origin() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::set_var("ALLOWED_ORIGINS", "http://valid.test,\ninvalid") };
        let err = match allowed_origins_from_env() {
            Ok(_) => panic!("invalid cors origin must fail closed"),
            Err(err) => err,
        };
        unsafe { std::env::remove_var("ALLOWED_ORIGINS") };
        assert!(err.to_string().contains("Invalid CORS origin"));
    }

    #[test]
    fn allowed_origins_from_env_fails_closed_on_wildcard_origin() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::set_var("ALLOWED_ORIGINS", "*") };
        let err = match allowed_origins_from_env() {
            Ok(_) => panic!("wildcard cors origin must fail closed"),
            Err(err) => err,
        };
        unsafe { std::env::remove_var("ALLOWED_ORIGINS") };
        assert!(
            err.to_string()
                .contains("Wildcard CORS origin is forbidden")
        );
    }

    #[test]
    fn allowed_origins_from_env_accepts_explicit_origin() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var(
                "ALLOWED_ORIGINS",
                "https://app.deve.com,http://127.0.0.1:3000",
            )
        };
        let origins = allowed_origins_from_env().expect("explicit origins should parse");
        unsafe { std::env::remove_var("ALLOWED_ORIGINS") };

        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0], "https://app.deve.com");
        assert_eq!(origins[1], "http://127.0.0.1:3000");
    }
}
