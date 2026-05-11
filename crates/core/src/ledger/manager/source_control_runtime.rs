//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract

use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;
use crate::ledger::traits::RepoSelector;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo, diff, pending_fs,
};
use anyhow::Result;
use std::collections::HashSet;

use super::source_control_target_resolution::change_identity_key;

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

    pub(crate) fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.manager
            .run_on_local_repo(repo_name, source_control::list_staged)
    }

    pub(crate) fn list_pending_fs_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.manager.run_on_local_repo(repo_name, |db| {
            let entries = pending_fs::list_all(db)?;
            Ok(entries
                .into_iter()
                .map(|entry| ChangeEntry {
                    path: entry.path,
                    renamed_from: entry.renamed_from,
                    doc_id: entry.doc_id,
                    status: entry.change_type,
                    has_conflict: entry.has_conflict,
                })
                .collect())
        })
    }

    pub(crate) fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        let staged = self.list_staged_in_local_repo(repo_name)?;
        let staged_keys: HashSet<_> = staged.iter().map(change_identity_key).collect();
        let mut changes = staged;
        changes.extend(
            self.list_pending_fs_in_local_repo(repo_name)?
                .into_iter()
                .filter(|entry| !staged_keys.contains(&change_identity_key(entry))),
        );
        Ok(changes)
    }

    pub(crate) fn diff_doc_path_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<String> {
        let target = self
            .manager
            .tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.diff_doc_target_in_local_repo(repo_name, &target)
    }

    pub(crate) fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        let (path, old_content, new_content) = self
            .manager
            .workdir_diff_inputs_for_target_in_local_repo(repo_name, target)?;
        Ok(diff::unified_diff(&old_content, &new_content, &path))
    }

    pub(crate) fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        source_control::validate_tables(self.manager.local_db.as_ref()).map_err(|err| {
            anyhow::anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                self.manager.local_repo_name(),
                err
            )
        })?;
        source_control::get_committed_content(&self.manager.local_db, doc_id)
    }

    pub(crate) fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<ChangeStatus> {
        source_control::detect_change(committed, current)
    }

    pub(crate) fn list_commits_in_local_repo(
        &self,
        repo_name: &str,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.manager
            .run_on_local_repo(repo_name, |db| source_control::list_commits(db, limit))
    }

    pub(crate) fn diff_commits_in_local_repo(
        &self,
        repo_name: &str,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let commit_a = commit_a_id.map(str::to_owned);
        let commit_b = commit_b_id.to_owned();
        self.manager.run_on_local_repo(repo_name, |db| {
            crate::source_control::commit_diff::compare_commits(db, commit_a.as_deref(), &commit_b)
        })
    }

    pub(crate) fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.list_pending_fs_in_local_repo(&repo_name)
    }

    pub(crate) fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.list_staged_in_local_repo(&repo_name)
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
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.list_changes_in_local_repo(&repo_name)
    }

    pub(crate) fn diff_doc_path_in_repo(
        &self,
        repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> Result<String> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.diff_doc_target_in_local_repo(&repo_name, target)
    }

    pub(crate) fn list_commits_in_repo(
        &self,
        repo: &RepoSelector,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.list_commits_in_local_repo(&repo_name, limit)
    }

    pub(crate) fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let repo_name = self.resolve_local_repo_for_execution(repo)?;
        self.diff_commits_in_local_repo(&repo_name, commit_a_id, commit_b_id)
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
