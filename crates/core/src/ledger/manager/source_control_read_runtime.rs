//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! Source Control read/query runtime.

use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeDomain, ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo, diff, ledger_dirty,
    pending_fs,
};
use anyhow::Result;
use std::collections::HashSet;

use super::source_control_target_resolution::change_identity_key;

pub(crate) struct SourceControlReadRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> SourceControlReadRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
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
                    domain: ChangeDomain::WorkingDirectory,
                    base_seq: None,
                    target_seq: None,
                })
                .collect())
        })
    }

    pub(crate) fn list_confirmed_ledger_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)
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
        changes.extend(self.list_confirmed_ledger_in_local_repo(repo_name)?);
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
        if target.domain == Some(ChangeDomain::ConfirmedLedger) {
            return self.manager.run_on_local_repo(repo_name, |db| {
                ledger_dirty::diff_confirmed_target(db, target)
            });
        }
        let (path, old_content, new_content) = self
            .manager
            .workdir_diff_inputs_for_target_in_local_repo(repo_name, target)?;
        Ok(diff::unified_diff(&old_content, &new_content, &path))
    }

    pub(crate) fn doc_diff_payload_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(Option<DocId>, String, String, String)> {
        if target.domain == Some(ChangeDomain::ConfirmedLedger) {
            let file = self.manager.run_on_local_repo(repo_name, |db| {
                ledger_dirty::confirmed_target_file(db, target)
            })?;
            return Ok((file.doc_id, file.path, file.old_content, file.new_content));
        }
        self.manager
            .workdir_diff_payload_for_target_in_local_repo(repo_name, target)
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
}
