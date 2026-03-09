// apps/cli/src/server/source_control_proxy.rs
//! # Source Control Remote Proxy

use anyhow::Result;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::DocId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use serde_json::json;

pub struct RemoteSourceControlApi {
    base_url: String,
    client: reqwest::Client,
}

impl RemoteSourceControlApi {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    fn with_repo_query(
        mut req: reqwest::RequestBuilder,
        repo: &RepoSelector,
    ) -> reqwest::RequestBuilder {
        if let Some(repo_id) = repo.repo_id {
            req = req.query(&[("repo_id", repo_id.to_string())]);
        }
        if let Some(repo_name) = &repo.repo_name {
            req = req.query(&[("repo_name", repo_name)]);
        }
        req
    }
}

/// 在异步上下文中安全执行阻塞 HTTP 请求
///
/// # 不变量
/// - 使用 `block_in_place` 避免在 Tokio 工作线程上死锁
fn block_on_safe<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

impl Repository for RemoteSourceControlApi {
    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>> {
        let url = format!("{}/api/repo/docs", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .send()
                .await?
                .json::<Vec<(DocId, String)>>()
                .await
        })?;
        Ok(res)
    }

    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        let url = format!("{}/api/repo/doc", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .query(&[("doc_id", doc_id.to_string())])
                .send()
                .await?
                .text()
                .await
        })?;
        Ok(res)
    }

    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let url = format!("{}/api/sc/pending", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .send()
                .await?
                .json::<Vec<ChangeEntry>>()
                .await
        })?;
        Ok(res)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let url = format!("{}/api/sc/stage-pending", self.base_url);
        block_on_safe(async {
            self.client
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

    fn discard_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let url = format!("{}/api/sc/discard-pending", self.base_url);
        block_on_safe(async {
            self.client
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

    fn unstage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let url = format!("{}/api/sc/unstage", self.base_url);
        block_on_safe(async {
            self.client
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

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let url = format!("{}/api/sc/status", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .send()
                .await?
                .json::<Vec<ChangeEntry>>()
                .await
        })?;
        Ok(res)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String> {
        let url = format!("{}/api/sc/diff", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .query(&[("path", path)])
                .send()
                .await?
                .text()
                .await
        })?;
        Ok(res)
    }

    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let url = format!("{}/api/sc/stage", self.base_url);
        block_on_safe(async {
            self.client
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

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        let url = format!("{}/api/sc/commits", self.base_url);
        let res = block_on_safe(async {
            Self::with_repo_query(self.client.get(&url), repo)
                .query(&[("limit", limit.to_string())])
                .send()
                .await?
                .json::<Vec<CommitInfo>>()
                .await
        })?;
        Ok(res)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let url = format!("{}/api/sc/commit-diff", self.base_url);
        let res = block_on_safe(async {
            let mut req = Self::with_repo_query(self.client.get(&url), repo)
                .query(&[("commit_b", commit_b_id)]);
            if let Some(commit_a) = commit_a_id {
                req = req.query(&[("commit_a", commit_a)]);
            }
            req.send().await?.json::<Vec<CommitFileDiff>>().await
        })?;
        Ok(res)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let url = format!("{}/api/sc/commit", self.base_url);
        let res = block_on_safe(async {
            self.client
                .post(&url)
                .json(&json!({
                    "message": message,
                    "repo_id": repo.repo_id.map(|id| id.to_string()),
                    "repo_name": repo.repo_name.clone(),
                }))
                .send()
                .await?
                .error_for_status()?
                .json::<CommitInfo>()
                .await
        })?;
        Ok(res)
    }
}
