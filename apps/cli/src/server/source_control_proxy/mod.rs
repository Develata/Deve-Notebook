// apps/cli/src/server/source_control_proxy/mod.rs
//! # Source Control Remote Proxy
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

mod client;
mod commits;
mod http;
mod mutations;
mod queries;

use anyhow::Result;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, SourceControlApi};

#[derive(Clone)]
pub struct RemoteSourceControlApi {
    base_url: String,
    client: reqwest::Client,
}

impl RemoteSourceControlApi {
    pub fn new(base_url: String) -> Self {
        let client = client::build_client(&base_url);
        Self { base_url, client }
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
        queries::list_docs(self, repo)
    }

    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        queries::get_doc_content(self, repo, doc_id)
    }
}

impl SourceControlApi for RemoteSourceControlApi {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        queries::list_pending(self, repo)
    }

    fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        queries::list_staged(self, repo)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        mutations::stage_pending(self, repo, target)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        mutations::discard_pending(self, repo, target)
    }

    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        mutations::unstage_file(self, repo, target)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        queries::list_changes(self, repo)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String> {
        queries::diff_doc_path(self, repo, target)
    }

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        commits::list_commits(self, repo, limit)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        commits::diff_commits(self, repo, commit_a_id, commit_b_id)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        commits::commit_staged(self, repo, message)
    }
}
