use super::RemoteSourceControlApi;
use anyhow::Result;
use deve_core::ledger::traits::RepoSelector;
use serde_json::json;

pub(super) fn stage_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    path: &str,
) -> Result<()> {
    post_path(api, repo, "/api/sc/stage-pending", path)
}

pub(super) fn discard_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    path: &str,
) -> Result<()> {
    post_path(api, repo, "/api/sc/discard-pending", path)
}

pub(super) fn unstage_file(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    path: &str,
) -> Result<()> {
    post_path(api, repo, "/api/sc/unstage", path)
}

pub(super) fn stage_file(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    path: &str,
) -> Result<()> {
    post_path(api, repo, "/api/sc/stage", path)
}

fn post_path(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    route: &str,
    path: &str,
) -> Result<()> {
    let url = format!("{}{}", api.base_url, route);
    super::block_on_safe(async {
        api.client
            .post(&url)
            .json(&json!({
                "path": path,
                "repo_id": repo.repo_id.map(|id| id.to_string()),
                "repo_name": repo.repo_name.clone(),
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok::<(), reqwest::Error>(())
    })?;
    Ok(())
}
