use anyhow::Result;
use deve_core::protocol::{ServerError, ServerErrorCode};
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::fmt;

/// 远端 HTTP 解码边界。
///
/// # 不变量
/// - 非 2xx 响应必须先被规范化为 `ServerError`，再离开 proxy 边界。
/// - 成功体仅在确认 2xx 后才允许按业务 JSON 或文本继续解析。
pub(super) async fn send_json<T: DeserializeOwned>(req: RequestBuilder) -> Result<T> {
    Ok(ensure_success(req.send().await?).await?.json::<T>().await?)
}

pub(super) async fn send_text(req: RequestBuilder) -> Result<String> {
    Ok(ensure_success(req.send().await?).await?.text().await?)
}

pub(super) async fn send_empty(req: RequestBuilder) -> Result<()> {
    ensure_success(req.send().await?).await?;
    Ok(())
}

async fn ensure_success(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    Err(read_error(response).await)
}

async fn read_error(response: Response) -> anyhow::Error {
    let status = response.status();
    match response.bytes().await {
        Ok(body) => ProxyServerError(decode_error(status, &body)).into(),
        Err(err) => err.into(),
    }
}

fn decode_error(status: StatusCode, body: &[u8]) -> ServerError {
    serde_json::from_slice::<ServerError>(body)
        .unwrap_or_else(|_| decode_plain_text_error(status, String::from_utf8_lossy(body).trim()))
}

fn decode_plain_text_error(status: StatusCode, raw_detail: &str) -> ServerError {
    let lower = raw_detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "active repository not selected",
            "multiple local repos exist",
            "no local repositories available",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoNotSelected, raw_detail);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "cannot bootstrap local repo while on remote branch",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
            "local repo not found for uuid",
            "local repo operation requested on remote branch",
            "local workspace path requested on remote branch",
            "local workspace root requested on remote branch",
            "scope mismatch",
            "stale scope nonce",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, raw_detail);
    }
    if lower.contains("tracked document projection missing") {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, raw_detail);
    }
    if contains_any(
        &lower,
        &[
            "broken repo entry",
            "broken local repo",
            "broken shadow repo",
            "broken shadow peer",
            "failed to walk local repo",
            "deserialize",
            "decode",
            "unexpected end",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, raw_detail);
    }
    if contains_any(
        &lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
            "failed to lock database",
        ],
    ) || status == StatusCode::SERVICE_UNAVAILABLE
    {
        return ServerError::with_detail(
            ServerErrorCode::StorageDbLocked,
            format_remote_detail(status, raw_detail),
        );
    }
    if lower.contains("path is not in pending_fs_ops") {
        return ServerError::with_detail(ServerErrorCode::ScPendingNotFound, raw_detail);
    }
    if lower.contains("path is not staged") {
        return ServerError::with_detail(ServerErrorCode::ScStagedNotFound, raw_detail);
    }
    if lower.contains("commit not found") {
        return ServerError::with_detail(ServerErrorCode::ScCommitNotFound, raw_detail);
    }
    if lower.contains("nothing to commit") {
        return ServerError::new(ServerErrorCode::ScNothingToCommit);
    }
    if contains_any(
        &lower,
        &[
            "doc not found",
            "document not found",
            "remote document not found",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScDocNotFound, raw_detail);
    }
    if lower.contains("local repo not found for name") {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, raw_detail);
    }
    if lower.contains("conflict") {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, raw_detail);
    }
    if status == StatusCode::NOT_FOUND {
        return ServerError::with_detail(
            ServerErrorCode::StorageNotFound,
            format_remote_detail(status, raw_detail),
        );
    }
    ServerError::with_detail(
        ServerErrorCode::RequestFailed,
        format_remote_detail(status, raw_detail),
    )
}

fn format_remote_detail(status: StatusCode, raw_detail: &str) -> String {
    if raw_detail.is_empty() {
        format!("remote source control request failed with HTTP {status}")
    } else {
        format!("remote source control request failed with HTTP {status}: {raw_detail}")
    }
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[derive(Debug)]
struct ProxyServerError(ServerError);

impl fmt::Display for ProxyServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(&self.0) {
            Ok(json) => f.write_str(&json),
            Err(_) => f.write_str(r#"{"code":"REQUEST_FAILED"}"#),
        }
    }
}

impl std::error::Error for ProxyServerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_server_error_json() {
        let err = decode_error(
            StatusCode::BAD_REQUEST,
            br#"{"code":"SC_PENDING_NOT_FOUND","detail":"missing"}"#,
        );
        assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
        assert_eq!(err.detail.as_deref(), Some("missing"));
    }

    #[test]
    fn wraps_plain_text_errors() {
        let err = decode_error(StatusCode::NOT_FOUND, b"notes/a.md");
        assert_eq!(err.code, ServerErrorCode::StorageNotFound);
        assert!(
            err.detail
                .as_deref()
                .is_some_and(|detail| detail.contains("404 Not Found"))
        );
    }

    #[test]
    fn maps_plain_text_pending_miss() {
        let err = decode_error(
            StatusCode::CONFLICT,
            b"Path is not in pending_fs_ops: notes/a.md",
        );
        assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
        assert_eq!(
            err.detail.as_deref(),
            Some("Path is not in pending_fs_ops: notes/a.md")
        );
    }

    #[test]
    fn maps_plain_text_repo_scope_drift() {
        let err = decode_error(
            StatusCode::CONFLICT,
            b"Repo selector mismatch: repo_id resolved to default, repo_name resolved to test",
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_plain_text_remote_bootstrap_drift() {
        let err = decode_error(
            StatusCode::CONFLICT,
            b"Cannot bootstrap local repo while on remote branch",
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_plain_text_broken_repo_entry_to_storage_persist_failed() {
        let err = decode_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            br#"Broken repo entry "/tmp/local/.redb" while listing repos: invalid file stem"#,
        );
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn maps_plain_text_stale_scope_nonce() {
        let err = decode_error(
            StatusCode::CONFLICT,
            b"Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7",
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_plain_text_legacy_projection_breakage() {
        let err = decode_error(
            StatusCode::CONFLICT,
            b"Tracked document projection missing for legacy-mapped path: notes/legacy.md",
        );
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
        assert_eq!(
            err.detail.as_deref(),
            Some("Tracked document projection missing for legacy-mapped path: notes/legacy.md")
        );
    }

    #[test]
    fn maps_plain_text_missing_local_repo_name() {
        let err = decode_error(StatusCode::NOT_FOUND, b"Local repo not found for name wiki");
        assert_eq!(err.code, ServerErrorCode::StorageNotFound);
        assert_eq!(
            err.detail.as_deref(),
            Some("Local repo not found for name wiki")
        );
    }
}
