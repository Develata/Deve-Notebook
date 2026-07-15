//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
//! Source Control commit orchestration runtime.

use crate::ledger::manager::commit_plan::CommitTarget;
#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::manager::git_mirror_queue_runtime;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::manager::{commit_plan, commit_preflight};
use crate::ledger::schema::LEDGER_OPS;
use crate::ledger::{ops, range};
use crate::models::DocId;
use crate::source_control::{
    ChangeEntry, ChangeStatus, CommitAuthorityFailure, CommitInfo, ExternalApplyOutcome,
    ExternalApplyReceipt, changes, commits, external_overlap, ledger_dirty, staging,
};
use anyhow::Result;
use redb::{Database, ReadableTable, WriteTransaction};
use std::collections::HashSet;

pub(crate) struct CommitRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> CommitRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn commit_source_control_changes(&self, message: &str) -> Result<CommitInfo> {
        self.commit_source_control_changes_in_local_repo(self.manager.local_repo_name(), message)
    }

    pub(crate) fn commit_source_control_changes_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        let repo_id = self.manager.run_on_local_repo(repo_name, |db| {
            RepoManager::read_repo_info_from_db(db)?
                .map(|info| info.uuid)
                .ok_or_else(|| anyhow::anyhow!("Repository metadata missing for {repo_name}"))
        })?;
        let commit = self
            .commit_source_control_authority_in_local_repo(repo_name, message)
            .map_err(CommitAuthorityFailure::into_error)?;
        #[cfg(not(target_arch = "wasm32"))]
        self.enqueue_git_mirror_projection_in_local_repo(repo_name, repo_id, &commit);
        Ok(commit)
    }

    pub(crate) fn commit_source_control_authority_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> std::result::Result<CommitInfo, CommitAuthorityFailure> {
        let prepared = self
            .prepare_source_control_commit_in_local_repo(repo_name)
            .map_err(CommitAuthorityFailure::NotCommitted)?;
        self.commit_source_control_authority_with_prepared_in_local_repo(
            repo_name, message, prepared,
        )
    }

    pub(crate) fn prepare_source_control_commit_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Option<crate::source_control::PreparedExternalApply>> {
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        let has_resolved_conflict_staged = staged.iter().any(|(_, entry)| entry.resolved_conflict);
        if has_resolved_conflict_staged && !staged.iter().all(|(_, entry)| entry.resolved_conflict)
        {
            anyhow::bail!(
                "Cannot commit mixed resolved-conflict and ordinary external staged changes"
            );
        }
        if has_resolved_conflict_staged {
            self.prepare_external_changes_in_local_repo(repo_name)
                .map(Some)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn commit_source_control_authority_with_prepared_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
        prepared_external: Option<crate::source_control::PreparedExternalApply>,
    ) -> std::result::Result<CommitInfo, CommitAuthorityFailure> {
        let message = message.trim();
        if message.is_empty() {
            return Err(CommitAuthorityFailure::NotCommitted(anyhow::anyhow!(
                "source control commit requires a non-empty message"
            )));
        }
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)
            .map_err(CommitAuthorityFailure::NotCommitted)?;
        let has_resolved_conflict_staged = staged.iter().any(|(_, entry)| entry.resolved_conflict);
        if has_resolved_conflict_staged && !staged.iter().all(|(_, entry)| entry.resolved_conflict)
        {
            return Err(CommitAuthorityFailure::NotCommitted(anyhow::anyhow!(
                "Cannot commit mixed resolved-conflict and ordinary external staged changes"
            )));
        }
        let (confirmed, external_apply) = if staged.is_empty() || !has_resolved_conflict_staged {
            if prepared_external.is_some() {
                return Err(CommitAuthorityFailure::NotCommitted(anyhow::anyhow!(
                    "Source control staging mode changed after commit preflight"
                )));
            }
            (
                self.manager
                    .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)
                    .map_err(CommitAuthorityFailure::NotCommitted)?,
                None,
            )
        } else {
            let prepared_external = prepared_external.ok_or_else(|| {
                CommitAuthorityFailure::NotCommitted(anyhow::anyhow!(
                    "Resolved-conflict commit requires prepared External Apply state"
                ))
            })?;
            let outcome = self
                .commit_prepared_external_changes_in_local_repo(repo_name, prepared_external)
                .map_err(CommitAuthorityFailure::NotCommitted)?;
            let receipt = outcome.receipt;
            let confirmed = self
                .manager
                .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)
                .map_err(|error| classify_commit_failure(Some(receipt.clone()), error))?;
            (confirmed, Some(receipt))
        };
        let commit_result = (|| -> Result<(CommitInfo, u32)> {
            if confirmed.is_empty() {
                anyhow::bail!("Nothing to commit: confirmed ledger changes are empty");
            }
            let final_confirmed = self
                .manager
                .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
            let doc_count = covered_doc_count(&final_confirmed);
            let (expected_base_seq, expected_ledger_head) = confirmed_range(&final_confirmed)?;
            let expected_parent_id = self.manager.run_on_local_repo(repo_name, |db| {
                let parent_id = commits::get_latest_id(db)?;
                let parent_seq = match parent_id.as_deref() {
                    Some(parent_id) => commits::get(db, parent_id)?.ledger_seq,
                    None => 0,
                };
                if parent_seq != expected_base_seq {
                    anyhow::bail!(
                        "Source control commit base changed before snapshot preparation: expected {}, observed {}",
                        expected_base_seq,
                        parent_seq
                    );
                }
                Ok(parent_id)
            })?;
            let commit = self.manager.run_on_local_repo(repo_name, |db| {
                let snapshots = plan_confirmed_commit_snapshots(&final_confirmed);
                persist_commit_state_atomically(
                    db,
                    &snapshots,
                    expected_ledger_head,
                    expected_parent_id.as_deref(),
                    message,
                    doc_count,
                )
            })?;
            Ok((commit, doc_count))
        })();
        let (commit, doc_count) =
            commit_result.map_err(|error| classify_commit_failure(external_apply, error))?;
        tracing::info!(
            "Committed {} files in {}: {}",
            doc_count,
            repo_name,
            message
        );
        Ok(commit)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn enqueue_git_mirror_projection_in_local_repo(
        &self,
        repo_name: &str,
        expected_repo_id: crate::models::RepoId,
        commit: &CommitInfo,
    ) {
        if !git_mirror_queue_runtime::binding_is_current(self.manager, repo_name, expected_repo_id)
        {
            return;
        }
        if let Err(err) = self.manager.run_on_local_repo(repo_name, |db| {
            let observed = RepoManager::read_repo_info_from_db(db)?
                .map(|info| info.uuid)
                .ok_or_else(|| anyhow::anyhow!("Repository metadata missing for {repo_name}"))?;
            if observed != expected_repo_id {
                anyhow::bail!(
                    "Git mirror queue repository identity changed: expected {expected_repo_id}, observed {observed}"
                );
            }
            crate::git_bridge::queue_deve_commit(db, expected_repo_id, commit)?;
            Ok(())
        }) {
            tracing::warn!(
                repo_name,
                deve_commit_id = %commit.id,
                error = %err,
                "Git mirror queue update failed after Deve commit; ledger commit is kept"
            );
        }
    }

    pub(crate) fn apply_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<ExternalApplyReceipt> {
        Ok(self
            .apply_external_changes_with_outcome_in_local_repo(repo_name)?
            .receipt)
    }

    pub(crate) fn apply_external_changes_with_outcome_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<ExternalApplyOutcome> {
        let prepared = self.prepare_external_changes_in_local_repo(repo_name)?;
        self.commit_prepared_external_changes_in_local_repo(repo_name, prepared)
    }

    pub(crate) fn prepare_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<crate::source_control::PreparedExternalApply> {
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        let staged_snapshot = staged.clone();
        let repo_id = self.manager.run_on_local_repo(repo_name, |db| {
            RepoManager::read_repo_info_from_db(db)?
                .map(|info| info.uuid)
                .ok_or_else(|| anyhow::anyhow!("Repository metadata missing for {repo_name}"))
        })?;
        let expected_ledger_head = self
            .manager
            .run_on_local_repo(repo_name, range::get_max_seq)?;
        let confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        let mut targets = commit_plan::build_targets(staged);
        if targets.is_empty() {
            anyhow::bail!("No external changes staged to apply");
        }
        targets.sort_by_key(|target| target.delete_only);
        ensure_external_targets_do_not_overlap_confirmed(&targets, &confirmed)?;
        commit_preflight::preflight_staged_commit_targets(self.manager, repo_name, &mut targets)?;
        self.manager.prepare_external_apply_in_local_repo(
            repo_name,
            targets,
            staged_snapshot,
            expected_ledger_head,
            repo_id,
        )
    }

    pub(crate) fn commit_prepared_external_changes_in_local_repo(
        &self,
        repo_name: &str,
        prepared: crate::source_control::PreparedExternalApply,
    ) -> Result<ExternalApplyOutcome> {
        let outcome = self
            .manager
            .commit_prepared_external_apply_in_local_repo(repo_name, prepared)?;
        tracing::info!(
            "Applied {} external changes to ledger in {}",
            outcome.receipt.applied_target_count,
            repo_name
        );
        Ok(outcome)
    }
}

fn classify_commit_failure(
    external_apply: Option<ExternalApplyReceipt>,
    error: anyhow::Error,
) -> CommitAuthorityFailure {
    match external_apply {
        Some(external_apply) => CommitAuthorityFailure::CommittedPartial {
            external_apply,
            error,
        },
        None => CommitAuthorityFailure::NotCommitted(error),
    }
}

fn ensure_external_targets_do_not_overlap_confirmed(
    targets: &[CommitTarget],
    confirmed: &[ChangeEntry],
) -> Result<()> {
    for target in targets {
        if target.allow_confirmed_overlap {
            continue;
        }
        if external_overlap::fields_overlap_any_confirmed(
            target.doc_id,
            &target.path,
            target.renamed_from.as_deref(),
            confirmed,
        ) {
            anyhow::bail!(
                "external change overlaps confirmed ledger changes: {}",
                target.path
            );
        }
    }
    Ok(())
}

fn covered_doc_count(confirmed: &[ChangeEntry]) -> u32 {
    let mut keys = HashSet::new();
    for entry in confirmed {
        insert_covered_key(&mut keys, entry.doc_id, &entry.path);
    }
    keys.len() as u32
}

fn insert_covered_key(keys: &mut HashSet<String>, doc_id: Option<DocId>, path: &str) {
    let key = match doc_id {
        Some(doc_id) => format!("doc:{doc_id}"),
        None => format!("path:{path}"),
    };
    keys.insert(key);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SnapshotMutation {
    Save(DocId),
    Remove(DocId),
}

fn confirmed_range(confirmed: &[ChangeEntry]) -> Result<(u64, u64)> {
    let first = confirmed
        .first()
        .ok_or_else(|| anyhow::anyhow!("confirmed ledger changes are empty"))?;
    let base = first
        .base_seq
        .ok_or_else(|| anyhow::anyhow!("confirmed ledger change lacks base sequence"))?;
    let target = first
        .target_seq
        .ok_or_else(|| anyhow::anyhow!("confirmed ledger change lacks target sequence"))?;
    if confirmed
        .iter()
        .any(|entry| entry.base_seq != Some(base) || entry.target_seq != Some(target))
    {
        anyhow::bail!("confirmed ledger changes do not share one commit range");
    }
    Ok((base, target))
}

fn plan_confirmed_commit_snapshots(confirmed: &[ChangeEntry]) -> Vec<SnapshotMutation> {
    let mut mutations = Vec::new();
    for entry in confirmed {
        let Some(doc_id) = entry.doc_id else {
            continue;
        };
        if entry.status == ChangeStatus::Deleted {
            mutations.push(SnapshotMutation::Remove(doc_id));
            continue;
        }
        mutations.push(SnapshotMutation::Save(doc_id));
    }
    mutations
}

fn persist_commit_state_atomically(
    db: &Database,
    snapshots: &[SnapshotMutation],
    expected_ledger_head: u64,
    expected_parent_id: Option<&str>,
    message: &str,
    doc_count: u32,
) -> Result<CommitInfo> {
    persist_commit_state_atomically_with_hook(
        db,
        snapshots,
        expected_ledger_head,
        expected_parent_id,
        message,
        doc_count,
        |_| Ok(()),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitStateStep {
    Snapshot(usize),
    BeforeAnchor,
}

fn persist_commit_state_atomically_with_hook<F>(
    db: &Database,
    snapshots: &[SnapshotMutation],
    expected_ledger_head: u64,
    expected_parent_id: Option<&str>,
    message: &str,
    doc_count: u32,
    mut hook: F,
) -> Result<CommitInfo>
where
    F: FnMut(CommitStateStep) -> Result<()>,
{
    let write_txn = db.begin_write()?;
    ensure_commit_preflight_current(&write_txn, expected_ledger_head, expected_parent_id)?;
    for (index, snapshot) in snapshots.iter().enumerate() {
        match snapshot {
            SnapshotMutation::Save(doc_id) => {
                let entries = ops::get_ops_from_txn(&write_txn, *doc_id)?;
                let facts: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
                let content = crate::state::reconstruct_content(&facts);
                changes::save_snapshot_in_txn(&write_txn, *doc_id, &content)?;
            }
            SnapshotMutation::Remove(doc_id) => {
                changes::remove_snapshot_in_txn(&write_txn, *doc_id)?;
            }
        }
        hook(CommitStateStep::Snapshot(index))?;
    }
    hook(CommitStateStep::BeforeAnchor)?;
    let commit = commits::create_in_txn(&write_txn, message, doc_count, expected_ledger_head)?;
    write_txn.commit()?;
    Ok(commit)
}

fn ensure_commit_preflight_current(
    write_txn: &WriteTransaction,
    expected_ledger_head: u64,
    expected_parent_id: Option<&str>,
) -> Result<()> {
    let ledger_head = write_txn
        .open_table(LEDGER_OPS)?
        .last()?
        .map(|(seq, _)| seq.value())
        .unwrap_or(0);
    if ledger_head != expected_ledger_head {
        anyhow::bail!(
            "Source control commit ledger head changed before atomic write: expected {}, observed {}",
            expected_ledger_head,
            ledger_head
        );
    }
    let parent_id = commits::get_latest_id_in_txn(write_txn)?;
    if parent_id.as_deref() != expected_parent_id {
        anyhow::bail!(
            "Source control commit parent changed before atomic write: expected {:?}, observed {:?}",
            expected_parent_id,
            parent_id
        );
    }
    Ok(())
}

impl RepoManager {
    pub(crate) fn commit_runtime(&self) -> CommitRuntime<'_> {
        CommitRuntime::new(self)
    }
}

#[cfg(test)]
mod atomic_tests;
