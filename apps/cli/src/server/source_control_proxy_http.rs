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
    serde_json::from_slice::<ServerError>(body).unwrap_or_else(|_| {
        let detail = String::from_utf8_lossy(body).trim().to_string();
        let detail = if detail.is_empty() {
            format!("remote source control request failed with HTTP {status}")
        } else {
            format!("remote source control request failed with HTTP {status}: {detail}")
        };
        ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
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
        assert_eq!(err.code, ServerErrorCode::RequestFailed);
        assert!(
            err.detail
                .as_deref()
                .is_some_and(|detail| detail.contains("404 Not Found"))
        );
    }
}
