use super::{RemoteSourceControlApi, http};
use anyhow::Result;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::DocId;
use deve_core::source_control::ChangeEntry;

pub(super) fn list_docs(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
) -> Result<Vec<(DocId, String)>> {
    let url = format!("{}/api/repo/docs", api.base_url);
    let res = super::block_on_safe(async {
        http::send_json(RemoteSourceControlApi::with_repo_query(
            api.client.get(&url),
            repo,
        ))
        .await
    })?;
    Ok(res)
}

pub(super) fn get_doc_content(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    doc_id: DocId,
) -> Result<String> {
    let url = format!("{}/api/repo/doc", api.base_url);
    let res = super::block_on_safe(async {
        http::send_text(
            RemoteSourceControlApi::with_repo_query(api.client.get(&url), repo)
                .query(&[("doc_id", doc_id.to_string())]),
        )
        .await
    })?;
    Ok(res)
}

pub(super) fn list_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
) -> Result<Vec<ChangeEntry>> {
    get_changes(api, repo, "/api/sc/pending")
}

pub(super) fn list_changes(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
) -> Result<Vec<ChangeEntry>> {
    get_changes(api, repo, "/api/sc/status")
}

pub(super) fn diff_doc_path(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    path: &str,
) -> Result<String> {
    let url = format!("{}/api/sc/diff", api.base_url);
    let res = super::block_on_safe(async {
        http::send_text(
            RemoteSourceControlApi::with_repo_query(api.client.get(&url), repo)
                .query(&[("path", path)]),
        )
        .await
    })?;
    Ok(res)
}

fn get_changes(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    route: &str,
) -> Result<Vec<ChangeEntry>> {
    let url = format!("{}{}", api.base_url, route);
    let res = super::block_on_safe(async {
        http::send_json(RemoteSourceControlApi::with_repo_query(
            api.client.get(&url),
            repo,
        ))
        .await
    })?;
    Ok(res)
}
