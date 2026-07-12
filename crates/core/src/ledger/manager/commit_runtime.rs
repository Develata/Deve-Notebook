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
    ChangeEntry, ChangeStatus, CommitInfo, changes, commits, external_overlap, ledger_dirty,
    staging,
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
        let message = message.trim();
        if message.is_empty() {
            anyhow::bail!("source control commit requires a non-empty message");
        }
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
        let confirmed = if staged.is_empty() || !has_resolved_conflict_staged {
            self.manager
                .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?
        } else {
            self.apply_external_changes_in_local_repo(repo_name)?
        };
        if confirmed.is_empty() {
            anyhow::bail!("Nothing to commit: confirmed ledger changes are empty");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let git_mirror_repo_id = git_mirror_queue_runtime::queue_repo_id(self.manager, repo_name);

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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(repo_id) = git_mirror_repo_id
            && let Err(err) = self.manager.run_on_local_repo(repo_name, |db| {
                crate::git_bridge::queue_deve_commit(db, repo_id, &commit)?;
                Ok(())
            })
        {
            tracing::warn!(
                repo_name,
                deve_commit_id = %commit.id,
                error = %err,
                "Git mirror queue update failed after Deve commit; ledger commit is kept"
            );
        }
        tracing::info!(
            "Committed {} files in {}: {}",
            doc_count,
            repo_name,
            message
        );
        Ok(commit)
    }

    pub(crate) fn apply_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        let staged_snapshot = staged.clone();
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
        self.manager
            .apply_external_targets_atomically_in_local_repo(
                repo_name,
                &targets,
                &staged_snapshot,
                expected_ledger_head,
            )?;

        let final_confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        tracing::info!(
            "Applied {} external changes to ledger in {}",
            targets.len(),
            repo_name
        );
        Ok(final_confirmed)
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
