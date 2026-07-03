//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! Source Control facade runtime.

use crate::config::GitBridgeMode;
use crate::ledger::manager::source_control_read_runtime::SourceControlReadRuntime;
use crate::ledger::manager::source_control_write_runtime::SourceControlWriteRuntime;
use crate::ledger::manager::types::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo};
use anyhow::Result;

pub(crate) struct SourceControlRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> SourceControlRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    fn read(&self) -> SourceControlReadRuntime<'a> {
        SourceControlReadRuntime::new(self.manager)
    }

    fn write(&self) -> SourceControlWriteRuntime<'a> {
        SourceControlWriteRuntime::new(self.manager)
    }

    pub(crate) fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.read().list_staged_in_local_repo(repo_name)
    }

    pub(crate) fn list_pending_fs_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.read().list_pending_fs_in_local_repo(repo_name)
    }

    pub(crate) fn list_confirmed_ledger_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.read().list_confirmed_ledger_in_local_repo(repo_name)
    }

    pub(crate) fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.read().list_changes_in_local_repo(repo_name)
    }

    pub(crate) fn diff_doc_path_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<String> {
        self.read().diff_doc_path_in_local_repo(repo_name, path)
    }

    pub(crate) fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        self.read().diff_doc_target_in_local_repo(repo_name, target)
    }

    pub(crate) fn doc_diff_payload_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(Option<DocId>, String, String, String)> {
        self.read()
            .doc_diff_payload_for_target_in_local_repo(repo_name, target)
    }

    pub(crate) fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        self.read().get_committed_content(doc_id)
    }

    pub(crate) fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<ChangeStatus> {
        self.read().detect_change(committed, current)
    }

    pub(crate) fn list_commits_in_local_repo(
        &self,
        repo_name: &str,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.read().list_commits_in_local_repo(repo_name, limit)
    }

    pub(crate) fn diff_commits_in_local_repo(
        &self,
        repo_name: &str,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.read()
            .diff_commits_in_local_repo(repo_name, commit_a_id, commit_b_id)
    }

    pub(crate) fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.write().unstage_file_in_local_repo(repo_name, path)
    }

    pub(crate) fn commit_staged_in_local_repo_with_git_bridge(
        &self,
        repo_name: &str,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        self.write()
            .commit_source_control_changes_in_local_repo_with_git_bridge(
                repo_name, message, git_bridge,
            )
    }

    pub(crate) fn commit_source_control_changes_in_local_repo_with_git_bridge(
        &self,
        repo_name: &str,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        self.write()
            .commit_source_control_changes_in_local_repo_with_git_bridge(
                repo_name, message, git_bridge,
            )
    }

    pub(crate) fn apply_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.write().apply_external_changes_in_local_repo(repo_name)
    }

    pub(crate) fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.write().stage_pending_in_local_repo(repo_name, path)
    }

    pub(crate) fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.write().discard_pending_in_local_repo(repo_name, path)
    }

    pub(crate) fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.write()
            .stage_pending_target_in_local_repo(repo_name, target)
    }

    pub(crate) fn stage_resolved_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.write()
            .stage_resolved_pending_target_in_local_repo(repo_name, target)
    }

    pub(crate) fn stage_resolved_pending_targets_in_local_repo(
        &self,
        repo_name: &str,
        targets: &[ScPathTarget],
    ) -> Result<()> {
        self.write()
            .stage_resolved_pending_targets_in_local_repo(repo_name, targets)
    }

    pub(crate) fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.write()
            .discard_pending_target_in_local_repo(repo_name, target)
    }

    pub(crate) fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.write()
            .unstage_file_target_in_local_repo(repo_name, target)
    }
}

impl RepoManager {
    pub(crate) fn source_control_runtime(&self) -> SourceControlRuntime<'_> {
        SourceControlRuntime::new(self)
    }
}
