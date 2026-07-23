// apps/cli/src/server/setup.rs
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 08_auth#cors
//!   - 08_auth#security-headers
//!   - 18_release#runtime-observability
//!
//! 服务器启动辅助: CORS 配置、文件监视器

use anyhow::{Context, Result, anyhow};
use deve_core::config::RuntimeEnvironment;
use deve_core::protocol::ServerMessage;
use deve_core::sync::watcher::{RepoWatcherStart, WatcherRefresh, WatcherRefreshKind};

use axum::http::{Method, header};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// 按显式 runtime override 或环境变量构建 CORS 层；默认不信任任何跨站来源。
pub(super) fn build_cors_layer(
    _port: u16,
    allowed_origins_override: Option<&[String]>,
    runtime_environment: RuntimeEnvironment,
) -> Result<CorsLayer> {
    let origins = match allowed_origins_override {
        Some(origins) => allowed_origins_from_values(origins.iter().map(String::as_str))?,
        None => allowed_origins_from_env()?,
    };
    if runtime_environment.is_development() && !origins.is_empty() {
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

    allowed_origins_from_values(origins.split(',').map(str::trim))
}

fn allowed_origins_from_values<'a>(
    origins: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<axum::http::HeaderValue>> {
    origins
        .into_iter()
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

pub(super) fn write_main_port_hint(
    host_dir: &std::path::Path,
    hint: &crate::local_cli_proxy_contract::LocalCliOwnerHint,
) -> Result<()> {
    let Some(host_root) = host_dir.parent() else {
        return Err(anyhow!(
            "Host directory has no parent while writing main port hint"
        ));
    };
    let hint_path = host_root.join("main_port");
    let temp_path = host_root.join(format!(".main-port-{}.tmp", uuid::Uuid::new_v4()));
    let mut temp = deve_core::utils::fs::create_atomic_replace_temp(&temp_path)
        .with_context(|| format!("Failed to create main port hint temp: {temp_path:?}"))?;
    let result = (|| -> Result<()> {
        use std::io::Write;
        serde_json::to_writer(&mut temp, hint).context("Failed to encode main port hint")?;
        temp.write_all(b"\n")?;
        temp.sync_all()?;
        deve_core::utils::fs::replace_file_atomically(&temp, &temp_path, &hint_path)
            .with_context(|| format!("Failed to publish main port hint: {hint_path:?}"))?;
        deve_core::utils::fs::sync_directory(host_root)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

/// 启动每个本地 repo 的 watcher。
pub(super) fn file_watcher_starts(
    sync_manager: Arc<deve_core::sync::SyncManager>,
) -> Result<Vec<RepoWatcherStart>> {
    Ok(sync_manager
        .healthy_local_repo_ids_for_execution()?
        .into_iter()
        .map(|repo_id| RepoWatcherStart::new(sync_manager.clone(), repo_id, repo_id.to_string(), 1))
        .collect())
}

/// Build one generation-bound repo start. Refresh/failure callbacks are
/// deliberately attached by `WatcherSupervisor`, which owns slot routing.
pub(super) fn file_watcher_start(
    sync_manager: Arc<deve_core::sync::SyncManager>,
    repo_name: impl Into<String>,
    generation: u64,
) -> Result<RepoWatcherStart> {
    Ok(RepoWatcherStart::resolve(
        sync_manager,
        repo_name,
        generation,
    )?)
}

pub(super) fn watcher_refresh_message(refresh: WatcherRefresh) -> ServerMessage {
    ServerMessage::FsChangeDetected {
        repo_id: Some(refresh.repo_id()),
        branch: None,
        scope_nonce: None,
        path: refresh.path().to_owned(),
        change_type: match refresh.kind() {
            WatcherRefreshKind::Added => "added",
            WatcherRefreshKind::Modified => "modified",
            WatcherRefreshKind::Deleted => "deleted",
            WatcherRefreshKind::DirectoryChanged => "dir_changed",
        }
        .to_owned(),
        has_conflict: refresh.has_conflict(),
    }
}

#[cfg(test)]
fn validate_file_watcher_startup(workspace_root: &std::path::Path) -> Result<()> {
    std::fs::canonicalize(workspace_root)
        .map(|_| ())
        .map_err(anyhow::Error::from)
        .with_context(|| format!("Watcher startup preflight failed for {:?}", workspace_root))
}

#[cfg(test)]
mod tests {
    use super::{
        allowed_origins_from_env, allowed_origins_from_values, build_cors_layer,
        validate_file_watcher_startup, watcher_refresh_message, write_main_port_hint,
    };
    use deve_core::config::RuntimeEnvironment;
    use deve_core::protocol::ServerMessage;
    use deve_core::sync::watcher::{WatcherRefresh, WatcherRefreshKind};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn watcher_refresh_adapter_maps_all_domain_fields() {
        let repo_id = uuid::Uuid::new_v4();
        for (kind, expected) in [
            (WatcherRefreshKind::Added, "added"),
            (WatcherRefreshKind::Modified, "modified"),
            (WatcherRefreshKind::Deleted, "deleted"),
            (WatcherRefreshKind::DirectoryChanged, "dir_changed"),
        ] {
            let message =
                watcher_refresh_message(WatcherRefresh::new(repo_id, "notes/live.md", kind, true));

            match message {
                ServerMessage::FsChangeDetected {
                    repo_id: actual_repo_id,
                    branch,
                    scope_nonce,
                    path,
                    change_type,
                    has_conflict,
                } => {
                    assert_eq!(actual_repo_id, Some(repo_id));
                    assert_eq!(branch, None);
                    assert_eq!(scope_nonce, None);
                    assert_eq!(path, "notes/live.md");
                    assert_eq!(change_type, expected);
                    assert!(has_conflict);
                }
                other => panic!("unexpected watcher adapter message: {other:?}"),
            }
        }
    }

    #[test]
    fn write_main_port_hint_fails_closed_when_parent_is_not_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bad_root = dir.path().join("host-root");
        std::fs::write(&bad_root, "not-a-dir").expect("bad root file");

        let hint = crate::local_cli_proxy_contract::LocalCliOwnerHint::new(
            3001,
            "aaaaaaaaaaaa".into(),
            uuid::Uuid::from_u128(1),
        );
        let err = write_main_port_hint(&bad_root.join(".host"), &hint)
            .expect_err("invalid host root must fail closed");

        assert!(
            err.to_string()
                .contains("Failed to create main port hint temp")
                || err.to_string().contains("Not a directory")
        );
    }

    #[test]
    fn main_port_hint_is_deterministic_owner_bound_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let host_dir = dir.path().join(".host").join("keys");
        std::fs::create_dir_all(&host_dir).expect("host key dir");
        let hint = crate::local_cli_proxy_contract::LocalCliOwnerHint::new(
            3001,
            "aaaaaaaaaaaa".into(),
            uuid::Uuid::from_u128(7),
        );
        write_main_port_hint(&host_dir, &hint).expect("write owner hint");
        let bytes =
            std::fs::read(dir.path().join(".host").join("main_port")).expect("read owner hint");
        let decoded: crate::local_cli_proxy_contract::LocalCliOwnerHint =
            serde_json::from_slice(&bytes).expect("decode owner hint");
        assert_eq!(decoded, hint);
        assert!(decoded.is_valid());
    }

    #[test]
    fn validate_file_watcher_startup_fails_closed_on_missing_workspace_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = validate_file_watcher_startup(&dir.path().join("missing-workspace"))
            .expect_err("missing workspace root must fail closed");

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

    #[test]
    fn allowed_origins_from_values_accepts_runtime_override() {
        let origins = allowed_origins_from_values(["http://tauri.localhost", "tauri://localhost"])
            .expect("runtime origin override");

        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0], "http://tauri.localhost");
        assert_eq!(origins[1], "tauri://localhost");
    }

    #[test]
    fn cors_layer_accepts_development_runtime_without_reading_deve_env() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::set_var("DEVE_ENV", "production") };

        let _ = build_cors_layer(
            3001,
            Some(&["http://127.0.0.1:8080".to_string()]),
            RuntimeEnvironment::Development,
        )
        .expect("explicit dev runtime cors");

        assert_eq!(std::env::var("DEVE_ENV").as_deref(), Ok("production"));
        unsafe { std::env::remove_var("DEVE_ENV") };
    }
}
