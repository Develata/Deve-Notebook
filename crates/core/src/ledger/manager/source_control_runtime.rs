//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract

use crate::ledger::manager::types::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use anyhow::Result;

pub(crate) struct SourceControlRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> SourceControlRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    fn resolve_local_repo_for_execution(&self, repo: &RepoSelector) -> Result<String> {
        self.manager
            .repo_scope_runtime()
            .resolve_local_selector_for_execution(repo)
    }

    fn with_local_repo<T>(
        &self,
        repo: &RepoSelector,
        f: impl FnOnce(&RepoManager, &str) -> Result<T>,
    ) -> Result<T> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        f(self.manager, &repo_name)
    }

    pub(crate) fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.with_local_repo(repo, RepoManager::list_pending_fs_in_local_repo)
    }

    pub(crate) fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.with_local_repo(repo, RepoManager::list_staged_in_local_repo)
    }

    pub(crate) fn stage_pending_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.stage_pending_target_in_local_repo(repo_name, target)
        })
    }

    pub(crate) fn discard_pending_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.discard_pending_target_in_local_repo(repo_name, target)
        })
    }

    pub(crate) fn unstage_file_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.unstage_file_target_in_local_repo(repo_name, target)
        })
    }

    pub(crate) fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.with_local_repo(repo, RepoManager::list_changes_in_local_repo)
    }

    pub(crate) fn diff_doc_path_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<String> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.diff_doc_target_in_local_repo(repo_name, target)
        })
    }

    pub(crate) fn list_commits_in_repo(
        &self,
        repo: &RepoSelector,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.list_commits_in_local_repo(repo_name, limit)
        })
    }

    pub(crate) fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.diff_commits_in_local_repo(repo_name, commit_a_id, commit_b_id)
        })
    }

    pub(crate) fn commit_staged_in_repo(
        &self,
        repo: &RepoSelector,
        message: &str,
    ) -> Result<CommitInfo> {
        self.with_local_repo(repo, |manager, repo_name| {
            manager.commit_staged_in_local_repo(repo_name, message)
        })
    }
}

impl RepoManager {
    pub(crate) fn source_control_runtime(&self) -> SourceControlRuntime<'_> {
        SourceControlRuntime::new(self)
    }
}
