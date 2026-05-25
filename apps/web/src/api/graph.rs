//! plan_ref:
//!   - 17_tech_stack#graph-visualization
//!   - 04_repository#repo-scope-runtime
//!
//! Read-only repo graph projection API.

use super::native_http::api_url;
use super::query::encode_query_component;
use deve_core::graph::GraphProjection;
use deve_core::protocol::{ServerError, ServerErrorCode};
use gloo_net::http::Request;
use web_sys::RequestCredentials;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphProjectionFetchError {
    RequestFailed,
    DegradedProjectionRequired,
}

pub async fn fetch_graph_projection(
    repo_id: Option<String>,
) -> Result<GraphProjection, GraphProjectionFetchError> {
    let api = api_url(&graph_projection_url(repo_id.as_deref(), false));
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .send()
        .await
        .map_err(|_| GraphProjectionFetchError::RequestFailed)?;
    if !response.ok() {
        return Err(classify_graph_projection_error(response).await);
    }
    response
        .json::<GraphProjection>()
        .await
        .map_err(|_| GraphProjectionFetchError::RequestFailed)
}

async fn classify_graph_projection_error(
    response: gloo_net::http::Response,
) -> GraphProjectionFetchError {
    match response.json::<ServerError>().await {
        Ok(error) => graph_projection_error_from_server_error(&error),
        _ => GraphProjectionFetchError::RequestFailed,
    }
}

fn graph_projection_error_from_server_error(error: &ServerError) -> GraphProjectionFetchError {
    match error.code {
        ServerErrorCode::GraphDegradedProjectionRequired => {
            GraphProjectionFetchError::DegradedProjectionRequired
        }
        _ => GraphProjectionFetchError::RequestFailed,
    }
}

fn graph_projection_url(repo_id: Option<&str>, allow_degraded_projection: bool) -> String {
    let mut url = "/api/repo/graph".to_string();
    let mut separator = "?";
    if let Some(repo_id) = repo_id {
        url.push_str(separator);
        url.push_str("repo_id=");
        url.push_str(&encode_query_component(repo_id));
        separator = "&";
    }
    if allow_degraded_projection {
        url.push_str(separator);
        url.push_str("allow_degraded_projection=true");
    }
    url
}

#[cfg(test)]
mod tests {
    use super::{
        GraphProjectionFetchError, graph_projection_error_from_server_error, graph_projection_url,
    };
    use deve_core::protocol::{ServerError, ServerErrorCode};

    #[test]
    fn graph_projection_url_uses_repo_id_when_available() {
        assert_eq!(
            graph_projection_url(Some("repo-1"), false),
            "/api/repo/graph?repo_id=repo-1"
        );
        assert_eq!(graph_projection_url(None, false), "/api/repo/graph");
        assert_eq!(
            graph_projection_url(Some("repo-1"), true),
            "/api/repo/graph?repo_id=repo-1&allow_degraded_projection=true"
        );
    }

    #[test]
    fn graph_projection_url_encodes_repo_id_query_component() {
        assert_eq!(
            graph_projection_url(Some("repo 1&x=1/雪"), true),
            "/api/repo/graph?repo_id=repo%201%26x%3D1%2F%E9%9B%AA&allow_degraded_projection=true"
        );
    }

    #[test]
    fn graph_projection_error_detects_structured_degraded_projection_code() {
        assert_eq!(
            graph_projection_error_from_server_error(&ServerError::new(
                ServerErrorCode::GraphDegradedProjectionRequired
            )),
            GraphProjectionFetchError::DegradedProjectionRequired
        );
        assert_eq!(
            graph_projection_error_from_server_error(&ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                "Use --allow-degraded-projection to export from metadata fallback."
            )),
            GraphProjectionFetchError::RequestFailed
        );
    }
}
