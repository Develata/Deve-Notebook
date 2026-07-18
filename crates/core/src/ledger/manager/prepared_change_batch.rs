//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/projection#projection-contract
//!
//! Crate-private sealed authority capability for source-specific batch commits.
//! Callers can hold only an opaque source wrapper; there is no generic fact
//! vector or callback constructor.

pub(super) mod remote_import;

use super::{commit_structure_plan, structure_projection};
use crate::ledger::schema::LEDGER_OPS;
use crate::ledger::{RepoManager, inode_index, node_ops, ops, reconcile};
use crate::models::{DocId, FactActor, FileNodeId, PeerId, RepoId, StructureOp};
use crate::source_control::staging::{self, StagedEntry};
use crate::source_control::{ExternalApplyOutcome, ExternalApplyReceipt};
use anyhow::{Result, anyhow};
use redb::ReadableTable;

const EXTERNAL_APPLY_ACTOR: &str = "external_apply_to_ledger";

/// The actual authority capability. It is intentionally non-Clone and remains
/// inside `deve_core`; only source-specific opaque wrappers may carry it across
/// an application-layer mutation permit.
pub(crate) struct PreparedLedgerChangeBatch {
    repo_id: RepoId,
    expected_ledger_head: u64,
    targets: Vec<PreparedLedgerTarget>,
    source: PreparedLedgerChangeSource,
}

enum PreparedLedgerChangeSource {
    External(PreparedExternalBatch),
    RemoteImport(Box<remote_import::PreparedRemoteImportBatch>),
}

struct PreparedExternalBatch {
    staged_snapshot: Vec<(String, StagedEntry)>,
    changed_paths: Vec<String>,
}

pub(super) struct PreparedLedgerUpsert {
    pub(super) path: String,
    pub(super) doc_id: DocId,
    pub(super) content_ops: Vec<crate::models::Op>,
    pub(super) inode: Option<FileNodeId>,
}

pub(super) enum PreparedLedgerTarget {
    Upsert(PreparedLedgerUpsert),
    Delete { path: String, doc_id: Option<DocId> },
}

impl PreparedLedgerChangeBatch {
    pub(super) fn external(
        repo_id: RepoId,
        expected_ledger_head: u64,
        staged_snapshot: Vec<(String, StagedEntry)>,
        targets: Vec<PreparedLedgerTarget>,
        changed_paths: Vec<String>,
    ) -> Self {
        Self {
            repo_id,
            expected_ledger_head,
            targets,
            source: PreparedLedgerChangeSource::External(PreparedExternalBatch {
                staged_snapshot,
                changed_paths,
            }),
        }
    }

    pub(super) fn remote_import(
        repo_id: RepoId,
        expected_ledger_head: u64,
        targets: Vec<PreparedLedgerTarget>,
        context: crate::remote_import::apply::RemoteImportApplyContext,
        overlap_targets: Vec<remote_import::RemoteImportOverlapTarget>,
    ) -> Self {
        Self {
            repo_id,
            expected_ledger_head,
            targets,
            source: PreparedLedgerChangeSource::RemoteImport(Box::new(
                remote_import::PreparedRemoteImportBatch {
                    context,
                    overlap_targets,
                },
            )),
        }
    }
}

pub(super) fn commit_external(
    manager: &RepoManager,
    repo_name: &str,
    batch: PreparedLedgerChangeBatch,
) -> Result<ExternalApplyOutcome> {
    let PreparedLedgerChangeBatch {
        repo_id,
        expected_ledger_head,
        targets,
        source,
    } = batch;
    let external = match source {
        PreparedLedgerChangeSource::External(external) => external,
        PreparedLedgerChangeSource::RemoteImport(_) => {
            anyhow::bail!("Remote Import sealed batch cannot enter External Apply writer")
        }
    };
    let repo_scope = ops::local_repo_scope(repo_name);
    let peer_id = manager.local_peer_id().clone();
    let applied_target_count = u32::try_from(targets.len())
        .map_err(|_| anyhow!("external apply target count exceeds u32"))?;
    let affected_docs = affected_docs(&targets);
    ensure_repo_identity(manager, repo_name, repo_id, "External Apply")?;

    let receipt = manager.run_on_local_repo(repo_name, |db| {
        let write_txn = db.begin_write()?;
        ensure_ledger_head_unchanged(
            &write_txn,
            expected_ledger_head,
            "External Changes preflight",
            "Apply to Ledger",
        )?;
        staging::consume_exact_in_txn(&write_txn, &external.staged_snapshot)?;
        append_targets_to_txn(
            &write_txn,
            &peer_id,
            &repo_scope,
            EXTERNAL_APPLY_ACTOR,
            &targets,
        )?;
        let authority_head = ledger_head_in_txn(&write_txn)?;
        write_txn.commit()?;
        Ok(ExternalApplyReceipt {
            repo_id,
            authority_head: authority_head.into(),
            affected_docs,
            applied_target_count,
        })
    })?;
    Ok(ExternalApplyOutcome {
        receipt,
        changed_paths: external.changed_paths,
    })
}

fn affected_docs(targets: &[PreparedLedgerTarget]) -> Vec<DocId> {
    let mut docs = targets
        .iter()
        .filter_map(|target| match target {
            PreparedLedgerTarget::Upsert(target) => Some(target.doc_id),
            PreparedLedgerTarget::Delete { doc_id, .. } => *doc_id,
        })
        .collect::<Vec<_>>();
    docs.sort_by_key(|doc_id| doc_id.as_u128());
    docs.dedup();
    docs
}

fn ensure_repo_identity(
    manager: &RepoManager,
    repo_name: &str,
    expected_repo_id: RepoId,
    operation: &str,
) -> Result<()> {
    let observed_repo_id = manager
        .run_on_local_repo(repo_name, RepoManager::read_repo_info_from_db)?
        .map(|info| info.uuid)
        .ok_or_else(|| anyhow!("Repository metadata missing for {repo_name}"))?;
    if observed_repo_id != expected_repo_id {
        anyhow::bail!(
            "{operation} repository identity changed: expected {expected_repo_id}, observed {observed_repo_id}"
        );
    }
    Ok(())
}

fn ensure_ledger_head_unchanged(
    write_txn: &redb::WriteTransaction,
    expected_ledger_head: u64,
    source: &str,
    retry_operation: &str,
) -> Result<()> {
    let current = ledger_head_in_txn(write_txn)?;
    if current != expected_ledger_head {
        anyhow::bail!(
            "Ledger changed during {source}: expected head {expected_ledger_head}, current {current}; refresh and retry {retry_operation}"
        );
    }
    Ok(())
}

fn ledger_head_in_txn(write_txn: &redb::WriteTransaction) -> Result<u64> {
    Ok(write_txn
        .open_table(LEDGER_OPS)?
        .last()?
        .map(|(key, _)| key.value())
        .unwrap_or(0))
}

fn append_targets_to_txn(
    write_txn: &redb::WriteTransaction,
    peer_id: &PeerId,
    repo_scope: &str,
    actor: &str,
    targets: &[PreparedLedgerTarget],
) -> Result<()> {
    for target in targets {
        match target {
            PreparedLedgerTarget::Upsert(target) => {
                let plan = commit_structure_plan::plan_file_upsert_in_txn(
                    write_txn,
                    &target.path,
                    target.doc_id,
                )?;
                append_structure_ops_to_txn(write_txn, peer_id, repo_scope, actor, &plan.ops)?;
                reconcile::append_patch_to_txn(
                    write_txn,
                    target.doc_id,
                    peer_id,
                    actor,
                    repo_scope,
                    &target.content_ops,
                )?;
                if let Some(inode) = &target.inode {
                    inode_index::bind_docid_in_txn(write_txn, inode, target.doc_id)?;
                }
            }
            PreparedLedgerTarget::Delete { path, doc_id } => {
                let ops = commit_structure_plan::plan_delete_in_txn(write_txn, path, *doc_id)?
                    .map(|plan| plan.ops)
                    .unwrap_or_default();
                append_structure_ops_to_txn(write_txn, peer_id, repo_scope, actor, &ops)?;
            }
        }
    }
    Ok(())
}

fn append_structure_ops_to_txn(
    write_txn: &redb::WriteTransaction,
    peer_id: &PeerId,
    repo_scope: &str,
    actor: &str,
    structure_ops: &[StructureOp],
) -> Result<()> {
    let actor = FactActor::new(actor)?;
    for op in structure_ops {
        node_ops::append_generated_structure_op_to_txn(
            write_txn,
            peer_id.clone(),
            actor.clone(),
            op.clone(),
            chrono::Utc::now().timestamp_millis(),
            repo_scope,
        )?;
        structure_projection::apply_in_txn(write_txn, op)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_apply_rejects_ledger_head_changed_after_preflight() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = redb::Database::create(dir.path().join("head-gate.redb"))?;
        let write = db.begin_write()?;
        let _ = write.open_table(LEDGER_OPS)?;
        write.commit()?;

        let concurrent = db.begin_write()?;
        concurrent.open_table(LEDGER_OPS)?.insert(1, &[1][..])?;
        concurrent.commit()?;

        let apply = db.begin_write()?;
        let error = ensure_ledger_head_unchanged(&apply, 0, "test preflight", "the operation")
            .expect_err("changed ledger head must reject stale sealed batch");
        assert!(error.to_string().contains("expected head 0, current 1"));
        Ok(())
    }
}
