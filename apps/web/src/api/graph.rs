//! plan_ref:
//!   - 14_tech_stack#graph-visualization
//!   - 06_repository#repo-scope-runtime
//!
//! Read-only repo graph projection API.

use deve_core::graph::GraphProjection;
use gloo_net::http::Request;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphProjectionFetchError {
    RequestFailed,
}

pub async fn fetch_graph_projection(
    repo_id: Option<String>,
) -> Result<GraphProjection, GraphProjectionFetchError> {
    let response = Request::get(&graph_projection_url(repo_id.as_deref()))
        .send()
        .await
        .map_err(|_| GraphProjectionFetchError::RequestFailed)?;
    response
        .ok()
        .then_some(response)
        .ok_or(GraphProjectionFetchError::RequestFailed)?
        .json::<GraphProjection>()
        .await
        .map_err(|_| GraphProjectionFetchError::RequestFailed)
}

fn graph_projection_url(repo_id: Option<&str>) -> String {
    repo_id.map_or_else(
        || "/api/repo/graph".to_string(),
        |repo_id| format!("/api/repo/graph?repo_id={repo_id}"),
    )
}

#[cfg(test)]
mod tests {
    use super::graph_projection_url;

    #[test]
    fn graph_projection_url_uses_repo_id_when_available() {
        assert_eq!(
            graph_projection_url(Some("repo-1")),
            "/api/repo/graph?repo_id=repo-1"
        );
        assert_eq!(graph_projection_url(None), "/api/repo/graph");
    }
}
