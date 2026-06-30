//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design/index#context-action-surface
//!
//! Host file actions for the Web thin client.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::JsFuture;
use web_sys::RequestCredentials;

use super::native_http::api_url;
use super::query::encode_query_component;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostFileActionError {
    RequestBuild,
    RequestFailed,
    InvalidResponse,
    ClipboardUnavailable,
    ClipboardWriteFailed,
}

#[derive(Deserialize)]
struct HostFilePathResponse {
    absolute_path: String,
}

#[derive(Serialize)]
struct HostFileRevealRequest<'a> {
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<&'a str>,
}

pub(crate) async fn fetch_host_file_absolute_path(
    repo_id: Option<String>,
    path: &str,
) -> Result<String, HostFileActionError> {
    let api = api_url(&host_file_path_url(repo_id.as_deref(), path));
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .send()
        .await
        .map_err(|_| HostFileActionError::RequestFailed)?;
    if !response.ok() {
        return Err(HostFileActionError::RequestFailed);
    }
    response
        .json::<HostFilePathResponse>()
        .await
        .map(|payload| payload.absolute_path)
        .map_err(|_| HostFileActionError::InvalidResponse)
}

pub(crate) async fn copy_host_file_absolute_path_to_clipboard(
    repo_id: Option<String>,
    path: String,
) -> Result<String, HostFileActionError> {
    let absolute_path = fetch_host_file_absolute_path(repo_id, &path).await?;
    write_text_to_clipboard(&absolute_path).await?;
    Ok(absolute_path)
}

pub(crate) async fn reveal_host_file_in_system_explorer(
    repo_id: Option<String>,
    path: String,
) -> Result<(), HostFileActionError> {
    let api = api_url("/api/repo/host-file-reveal");
    let mut request = Request::post(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .header("Content-Type", "application/json")
        .json(&HostFileRevealRequest {
            path: &path,
            repo_id: repo_id.as_deref(),
        })
        .map_err(|_| HostFileActionError::RequestBuild)?
        .send()
        .await
        .map_err(|_| HostFileActionError::RequestFailed)?;
    response
        .ok()
        .then_some(())
        .ok_or(HostFileActionError::RequestFailed)
}

async fn write_text_to_clipboard(text: &str) -> Result<(), HostFileActionError> {
    let window = web_sys::window().ok_or(HostFileActionError::ClipboardUnavailable)?;
    let clipboard = window.navigator().clipboard();
    JsFuture::from(clipboard.write_text(text))
        .await
        .map(|_| ())
        .map_err(|_| HostFileActionError::ClipboardWriteFailed)
}

fn host_file_path_url(repo_id: Option<&str>, path: &str) -> String {
    let mut url = format!(
        "/api/repo/host-file-path?path={}",
        encode_query_component(path)
    );
    if let Some(repo_id) = repo_id {
        url.push_str("&repo_id=");
        url.push_str(&encode_query_component(repo_id));
    }
    url
}

#[cfg(test)]
mod tests {
    use super::host_file_path_url;

    #[test]
    fn host_file_path_url_uses_repo_id_when_available() {
        assert_eq!(
            host_file_path_url(Some("repo-1"), "notes/readme.md"),
            "/api/repo/host-file-path?path=notes%2Freadme.md&repo_id=repo-1"
        );
        assert_eq!(
            host_file_path_url(None, "notes/readme.md"),
            "/api/repo/host-file-path?path=notes%2Freadme.md"
        );
    }

    #[test]
    fn host_file_path_url_encodes_query_components() {
        assert_eq!(
            host_file_path_url(Some("repo 1&x=1"), "notes/雪.md"),
            "/api/repo/host-file-path?path=notes%2F%E9%9B%AA.md&repo_id=repo%201%26x%3D1"
        );
    }
}
