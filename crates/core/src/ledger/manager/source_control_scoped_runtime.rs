//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! Repo-selector Source Control runtime.

use crate::ledger::manager::types::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeEntry, CommitFileDiff, CommitInfo, ExternalApplyOutcome, ExternalApplyReceipt,
};
use anyhow::Result;

pub(crate) struct SourceControlScopedRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> SourceControlScopedRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    fn resolve_local_repo_for_execution(&self, repo: &RepoSelector) -> Result<String> {
        self.manager
            .repo_scope_runtime()
            .resolve_local_selector_for_execution(repo)
    }

    pub(crate) fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .list_pending_fs_in_local_repo(&repo_name)
    }

    pub(crate) fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .list_staged_in_local_repo(&repo_name)
    }

    pub(crate) fn stage_pending_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .stage_pending_target_in_local_repo(&repo_name, target)
    }

    pub(crate) fn discard_pending_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .discard_pending_target_in_local_repo(&repo_name, target)
    }

    pub(crate) fn unstage_file_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<()> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .unstage_file_target_in_local_repo(&repo_name, target)
    }

    pub(crate) fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .list_changes_in_local_repo(&repo_name)
    }

    pub(crate) fn diff_doc_path_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<String> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .diff_doc_target_in_local_repo(&repo_name, target)
    }

    pub(crate) fn list_commits_in_repo(
        &self,
        repo: &RepoSelector,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .list_commits_in_local_repo(&repo_name, limit)
    }

    pub(crate) fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .diff_commits_in_local_repo(&repo_name, commit_a_id, commit_b_id)
    }

    pub(crate) fn commit_source_control_changes_in_repo(
        &self,
        repo: &RepoSelector,
        message: &str,
    ) -> Result<CommitInfo> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .commit_source_control_changes_in_local_repo(&repo_name, message)
    }

    pub(crate) fn apply_external_changes_in_repo(
        &self,
        repo: &RepoSelector,
    ) -> Result<ExternalApplyReceipt> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .apply_external_changes_in_local_repo(&repo_name)
    }

    pub(crate) fn apply_external_changes_with_outcome_in_repo(
        &self,
        repo: &RepoSelector,
    ) -> Result<ExternalApplyOutcome> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.manager
            .source_control_runtime()
            .apply_external_changes_with_outcome_in_local_repo(&repo_name)
    }
}

impl RepoManager {
    pub(crate) fn source_control_scoped_runtime(&self) -> SourceControlScopedRuntime<'_> {
        SourceControlScopedRuntime::new(self)
    }
}
