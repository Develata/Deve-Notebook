use super::{RemoteSourceControlApi, http};
use anyhow::Result;
use deve_core::ledger::traits::RepoSelector;
use deve_core::source_control::{CommitFileDiff, CommitInfo};
use serde_json::json;

pub(super) fn list_commits(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    limit: u32,
) -> Result<Vec<CommitInfo>> {
    let url = format!("{}/api/sc/commits", api.base_url);
    let res = super::block_on_safe(async {
        http::send_json(
            RemoteSourceControlApi::with_repo_query(api.client.get(&url), repo)
                .query(&[("limit", limit.to_string())]),
        )
        .await
    })?;
    Ok(res)
}

pub(super) fn diff_commits(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> Result<Vec<CommitFileDiff>> {
    let url = format!("{}/api/sc/commit-diff", api.base_url);
    let res = super::block_on_safe(async {
        let mut req = RemoteSourceControlApi::with_repo_query(api.client.get(&url), repo)
            .query(&[("commit_b", commit_b_id)]);
        if let Some(commit_a) = commit_a_id {
            req = req.query(&[("commit_a", commit_a)]);
        }
        http::send_json(req).await
    })?;
    Ok(res)
}

pub(super) fn commit_staged(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    message: &str,
) -> Result<CommitInfo> {
    let url = format!("{}/api/sc/commit", api.base_url);
    let res = super::block_on_safe(async {
        http::send_json(api.client.post(&url).json(&json!({
            "message": message,
            "repo_id": repo.repo_id.map(|id| id.to_string()),
            "repo_name": repo.repo_name.clone(),
        })))
        .await
    })?;
    Ok(res)
}
