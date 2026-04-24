use anyhow::Result;
use deve_core::protocol::ServerError;
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::fmt;

#[path = "source_control_proxy_http_plain.rs"]
mod plain;
#[path = "source_control_proxy_http_target.rs"]
mod target;

pub(crate) use target::ProxyScOp;

/// 远端 HTTP 解码边界。
///
/// # 不变量
/// - 非 2xx 响应必须先被规范化为 `ServerError`，再离开 proxy 边界。
/// - 成功体仅在确认 2xx 后才允许按业务 JSON 或文本继续解析。
pub(super) async fn send_json<T: DeserializeOwned>(req: RequestBuilder) -> Result<T> {
    Ok(ensure_success(req.send().await?, None)
        .await?
        .json::<T>()
        .await?)
}

pub(super) async fn send_text(req: RequestBuilder) -> Result<String> {
    Ok(ensure_success(req.send().await?, None)
        .await?
        .text()
        .await?)
}

pub(super) async fn send_text_with_op(req: RequestBuilder, op: ProxyScOp) -> Result<String> {
    Ok(ensure_success(req.send().await?, Some(&op))
        .await?
        .text()
        .await?)
}

pub(super) async fn send_empty_with_op(req: RequestBuilder, op: ProxyScOp) -> Result<()> {
    ensure_success(req.send().await?, Some(&op)).await?;
    Ok(())
}

async fn ensure_success(response: Response, op: Option<&ProxyScOp>) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    Err(read_error(response, op).await)
}

async fn read_error(response: Response, op: Option<&ProxyScOp>) -> anyhow::Error {
    let status = response.status();
    match response.bytes().await {
        Ok(body) => ProxyServerError(decode_error_with_op(status, &body, op)).into(),
        Err(err) => err.into(),
    }
}

#[cfg(test)]
fn decode_error(status: StatusCode, body: &[u8]) -> ServerError {
    decode_error_with_op(status, body, None)
}

fn decode_error_with_op(status: StatusCode, body: &[u8], op: Option<&ProxyScOp>) -> ServerError {
    serde_json::from_slice::<ServerError>(body).unwrap_or_else(|_| {
        plain::decode_plain_text_error(status, String::from_utf8_lossy(body).trim(), op)
    })
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
