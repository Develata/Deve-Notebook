use super::{RemoteSourceControlApi, http};
use anyhow::Result;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use serde_json::json;

pub(super) fn stage_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(api, repo, "/api/sc/stage-pending", target)
}

pub(super) fn discard_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(api, repo, "/api/sc/discard-pending", target)
}

pub(super) fn unstage_file(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(api, repo, "/api/sc/unstage", target)
}

fn post_target(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    route: &str,
    target: &ScPathTarget,
) -> Result<()> {
    let url = format!("{}{}", api.base_url, route);
    super::block_on_safe(async {
        http::send_empty(api.client.post(&url).json(&json!({
            "path": target.path,
            "doc_id": target.doc_id.map(|id| id.to_string()),
            "repo_id": repo.repo_id.map(|id| id.to_string()),
            "repo_name": repo.repo_name.clone(),
        })))
        .await
    })?;
    Ok(())
}
