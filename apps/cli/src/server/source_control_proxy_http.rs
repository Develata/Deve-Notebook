use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_storage_corruption,
};
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
    if is_repo_not_selected(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoNotSelected, raw_detail);
    }
    if is_repo_context_invalid(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, raw_detail);
    }
    if is_storage_corruption(&lower) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, raw_detail);
    }
    if is_db_locked(&lower) || status == StatusCode::SERVICE_UNAVAILABLE {
        return ServerError::with_detail(
            ServerErrorCode::StorageDbLocked,
            format_remote_detail(status, raw_detail),
        );
    }
    if lower.contains("path is not in pending_fs_ops") {
        return ServerError::with_detail(ServerErrorCode::ScPendingNotFound, raw_detail);
    }
    if contains_any(
        &lower,
        &["ambiguous pending_fs target", "ambiguous staged target"],
    ) {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, raw_detail);
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
#[path = "source_control_proxy_http_test.rs"]
mod tests;
