//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
//! Atomic application of preflighted External Changes into ledger authority.

use crate::ledger::manager::commit_plan::CommitTarget;
use crate::ledger::schema::LEDGER_OPS;
use crate::ledger::{RepoManager, inode_index, node_ops, ops, reconcile};
use crate::models::{DocId, FactActor, FileNodeId, PeerId, StructureOp};
use crate::source_control::staging::{self, StagedEntry};
use crate::source_control::{
    ExternalApplyOutcome, ExternalApplyReceipt, PreparedExternalApply, PreparedExternalTarget,
    PreparedUpsert,
};
use crate::utils::fs::checked_exists;
use anyhow::{Result, anyhow};
use redb::ReadableTable;

use super::{commit_structure_plan, structure_projection};

const EXTERNAL_APPLY_SOURCE: &str = "external_apply_to_ledger";

impl RepoManager {
    pub(super) fn prepare_external_apply_in_local_repo(
        &self,
        repo_name: &str,
        targets: Vec<CommitTarget>,
        staged_snapshot: Vec<(String, StagedEntry)>,
        expected_ledger_head: u64,
        repo_id: crate::models::RepoId,
    ) -> Result<PreparedExternalApply> {
        let prepared = targets
            .iter()
            .map(|target| self.prepare_external_target(repo_name, target))
            .collect::<Result<Vec<_>>>()?;
        let mut changed_paths: Vec<_> = staged_snapshot
            .iter()
            .map(|(path, _)| path.clone())
            .collect();
        changed_paths.sort();
        changed_paths.dedup();
        Ok(PreparedExternalApply {
            repo_id,
            expected_ledger_head,
            staged_snapshot,
            targets: prepared,
            changed_paths,
        })
    }

    pub(super) fn commit_prepared_external_apply_in_local_repo(
        &self,
        repo_name: &str,
        prepared: PreparedExternalApply,
    ) -> Result<ExternalApplyOutcome> {
        let PreparedExternalApply {
            repo_id,
            expected_ledger_head,
            staged_snapshot,
            targets,
            changed_paths,
        } = prepared;
        let repo_scope = ops::local_repo_scope(repo_name);
        let peer_id = self.local_peer_id().clone();
        let applied_target_count = u32::try_from(targets.len())
            .map_err(|_| anyhow!("external apply target count exceeds u32"))?;
        let mut affected_docs: Vec<DocId> = targets
            .iter()
            .filter_map(|target| match target {
                PreparedExternalTarget::Upsert(target) => Some(target.doc_id),
                PreparedExternalTarget::Delete { doc_id, .. } => *doc_id,
            })
            .collect();
        affected_docs.sort_by_key(|doc_id| doc_id.as_u128());
        affected_docs.dedup();
        let observed_repo_id = self
            .run_on_local_repo(repo_name, RepoManager::read_repo_info_from_db)?
            .map(|info| info.uuid)
            .ok_or_else(|| anyhow!("Repository metadata missing for {repo_name}"))?;
        if observed_repo_id != repo_id {
            anyhow::bail!(
                "External Apply repository identity changed: expected {repo_id}, observed {observed_repo_id}"
            );
        }

        let receipt = self.run_on_local_repo(repo_name, |db| {
            let write_txn = db.begin_write()?;
            ensure_ledger_head_unchanged(&write_txn, expected_ledger_head)?;
            staging::consume_exact_in_txn(&write_txn, &staged_snapshot)?;
            for target in &targets {
                match target {
                    PreparedExternalTarget::Upsert(target) => {
                        let plan = commit_structure_plan::plan_file_upsert_in_txn(
                            &write_txn,
                            &target.path,
                            target.doc_id,
                        )?;
                        append_structure_ops_to_txn(&write_txn, &peer_id, &repo_scope, &plan.ops)?;
                        reconcile::append_patch_to_txn(
                            &write_txn,
                            target.doc_id,
                            &peer_id,
                            EXTERNAL_APPLY_SOURCE,
                            &repo_scope,
                            &target.content_ops,
                        )?;
                        if let Some(inode) = &target.inode {
                            inode_index::bind_docid_in_txn(&write_txn, inode, target.doc_id)?;
                        }
                    }
                    PreparedExternalTarget::Delete { path, doc_id } => {
                        let ops =
                            commit_structure_plan::plan_delete_in_txn(&write_txn, path, *doc_id)?
                                .map(|plan| plan.ops)
                                .unwrap_or_default();
                        append_structure_ops_to_txn(&write_txn, &peer_id, &repo_scope, &ops)?
                    }
                }
            }
            let authority_head = write_txn
                .open_table(LEDGER_OPS)?
                .last()?
                .map(|(seq, _)| seq.value())
                .unwrap_or(0);
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
            changed_paths,
        })
    }

    fn prepare_external_target(
        &self,
        repo_name: &str,
        target: &CommitTarget,
    ) -> Result<PreparedExternalTarget> {
        if target.delete_only {
            return Ok(PreparedExternalTarget::Delete {
                path: target.path.clone(),
                doc_id: target.doc_id,
            });
        }

        let content = target.validated_content.as_deref().ok_or_else(|| {
            anyhow!(
                "external apply target was not preflighted with content: {}",
                target.path
            )
        })?;
        let doc_id =
            commit_structure_plan::resolve_doc_id(self, repo_name, &target.path, target.doc_id)?;
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, entry)| entry).collect();
        let content_ops = reconcile::compute_reconcile_patch(&entries, content)?;
        let inode = self.external_target_inode(repo_name, &target.path)?;
        Ok(PreparedExternalTarget::Upsert(PreparedUpsert {
            path: target.path.clone(),
            doc_id,
            content_ops,
            inode,
        }))
    }

    fn external_target_inode(&self, repo_name: &str, path: &str) -> Result<Option<FileNodeId>> {
        let file_path = self.local_repo_workspace_path(repo_name, path)?;
        if !checked_exists(&file_path, "staged workspace path while binding inode")? {
            return Ok(None);
        }
        let file_id = file_id::get_file_id(&file_path)?;
        use crate::utils::hash::StableHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = StableHasher::new();
        file_id.hash(&mut hasher);
        Ok(Some(FileNodeId {
            id: hasher.finish() as u128,
        }))
    }
}

fn ensure_ledger_head_unchanged(
    write_txn: &redb::WriteTransaction,
    expected_ledger_head: u64,
) -> Result<()> {
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let current = ops.last()?.map(|(key, _)| key.value()).unwrap_or(0);
    if current != expected_ledger_head {
        anyhow::bail!(
            "Ledger changed during External Changes preflight: expected head {}, current {}; refresh and retry Apply to Ledger",
            expected_ledger_head,
            current
        );
    }
    Ok(())
}

fn append_structure_ops_to_txn(
    write_txn: &redb::WriteTransaction,
    peer_id: &PeerId,
    repo_scope: &str,
    structure_ops: &[StructureOp],
) -> Result<()> {
    for op in structure_ops {
        node_ops::append_generated_structure_op_to_txn(
            write_txn,
            peer_id.clone(),
            FactActor::new(EXTERNAL_APPLY_SOURCE)?,
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
    fn atomic_apply_rejects_ledger_head_changed_after_preflight() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = redb::Database::create(dir.path().join("head-gate.redb"))?;
        let write = db.begin_write()?;
        let _ = write.open_table(LEDGER_OPS)?;
        write.commit()?;

        let concurrent = db.begin_write()?;
        concurrent.open_table(LEDGER_OPS)?.insert(1, &[1][..])?;
        concurrent.commit()?;

        let apply = db.begin_write()?;
        let error = ensure_ledger_head_unchanged(&apply, 0)
            .expect_err("changed ledger head must reject stale prepared patch");
        assert!(error.to_string().contains("expected head 0, current 1"));
        Ok(())
    }
}
