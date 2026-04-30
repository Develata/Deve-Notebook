//! plan_ref:
//!   - 14_tech_stack#graph-visualization
//!   - 06_repository#repo-scope-runtime
//!
//! Read-only repo graph projection API.

use deve_core::graph::GraphProjection;
use deve_core::protocol::ServerError;
use gloo_net::http::Request;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphProjectionFetchError {
    RequestFailed,
    DegradedProjectionRequired,
}

pub async fn fetch_graph_projection(
    repo_id: Option<String>,
) -> Result<GraphProjection, GraphProjectionFetchError> {
    let response = Request::get(&graph_projection_url(repo_id.as_deref(), false))
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
        Ok(error) if graph_projection_requires_degraded_flag(error.detail.as_deref()) => {
            GraphProjectionFetchError::DegradedProjectionRequired
        }
        _ => GraphProjectionFetchError::RequestFailed,
    }
}

fn graph_projection_requires_degraded_flag(detail: Option<&str>) -> bool {
    detail
        .map(|detail| detail.contains("--allow-degraded-projection"))
        .unwrap_or(false)
}

fn graph_projection_url(repo_id: Option<&str>, allow_degraded_projection: bool) -> String {
    let mut url = "/api/repo/graph".to_string();
    let mut separator = "?";
    if let Some(repo_id) = repo_id {
        url.push_str(separator);
        url.push_str("repo_id=");
        url.push_str(repo_id);
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
    use super::{graph_projection_requires_degraded_flag, graph_projection_url};

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
    fn graph_projection_error_detects_degraded_projection_gate() {
        assert!(graph_projection_requires_degraded_flag(Some(
            "Use --allow-degraded-projection to export from metadata fallback."
        )));
        assert!(!graph_projection_requires_degraded_flag(Some(
            "repo context invalid"
        )));
        assert!(!graph_projection_requires_degraded_flag(None));
    }
}
