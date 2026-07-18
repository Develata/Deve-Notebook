//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 06_backup#remote-import-session-contract
//!   - 03_storage/projection#projection-contract
//!
//! Source-specific Remote Import preparation for the sealed ledger writer.

use super::prepared_change_batch::remote_import::RemoteImportOverlapTarget;
use super::prepared_change_batch::{
    PreparedLedgerChangeBatch, PreparedLedgerTarget, PreparedLedgerUpsert, remote_import,
};
use crate::ledger::{RepoManager, reconcile};
use crate::remote_import::apply::{PreparedRemoteImportApply, RemoteImportPreparedEntry};
use crate::remote_import::{
    RemoteImportApplyReceipt, RemoteImportChangeKind, RemoteImportDigest, RemoteImportError,
    RemoteImportResult,
};

impl RepoManager {
    pub(crate) fn prepare_remote_import_apply_in_local_repo(
        &self,
        repo_name: &str,
        prepared: PreparedRemoteImportApply,
    ) -> RemoteImportResult<PreparedLedgerChangeBatch> {
        let (context, entries) = prepared.into_parts();
        let repo_id = context.repo_id();
        let expected_head = context.expected_head().storage_key();
        if context.is_replay() {
            return Ok(PreparedLedgerChangeBatch::remote_import(
                repo_id,
                expected_head,
                Vec::new(),
                context,
                Vec::new(),
            ));
        }
        let mut targets = Vec::with_capacity(context.expected_mutation_count());
        let mut overlap_targets = Vec::with_capacity(entries.len());
        for entry in entries {
            let (target, overlap) = self.prepare_remote_import_entry(repo_name, entry)?;
            if let Some(target) = target {
                targets.push(target);
            }
            overlap_targets.push(overlap);
        }
        Ok(PreparedLedgerChangeBatch::remote_import(
            repo_id,
            expected_head,
            targets,
            context,
            overlap_targets,
        ))
    }

    pub(crate) fn commit_prepared_remote_import_apply_in_local_repo(
        &self,
        repo_name: &str,
        prepared: PreparedLedgerChangeBatch,
    ) -> RemoteImportResult<RemoteImportApplyReceipt> {
        remote_import::commit_remote_import(self, repo_name, prepared)
    }

    fn prepare_remote_import_entry(
        &self,
        repo_name: &str,
        entry: RemoteImportPreparedEntry,
    ) -> RemoteImportResult<(Option<PreparedLedgerTarget>, RemoteImportOverlapTarget)> {
        if !entry.blockers.is_empty() {
            return Err(RemoteImportError::ApplyFailed(
                "blocked Remote Import entry escaped source-specific preparation".to_string(),
            ));
        }
        let current_doc = self
            .get_tracked_docid_in_local_repo(repo_name, &entry.path)
            .map_err(RemoteImportError::apply_failed)?;
        let current_content = match current_doc {
            Some(doc_id) => {
                let entries = self
                    .get_local_ops_in_local_repo(repo_name, doc_id)
                    .map_err(RemoteImportError::apply_failed)?
                    .into_iter()
                    .map(|(_, entry)| entry)
                    .collect::<Vec<_>>();
                Some(crate::state::reconstruct_content(&entries))
            }
            None => None,
        };
        validate_change_kind(&entry, current_content.as_deref())?;
        let overlap = RemoteImportOverlapTarget {
            path: entry.path.clone(),
            doc_id: current_doc,
        };
        if entry.change_kind == RemoteImportChangeKind::Unchanged {
            return Ok((None, overlap));
        }
        let doc_id = current_doc.unwrap_or_else(crate::models::DocId::new);
        let existing_entries = self
            .get_local_ops_in_local_repo(repo_name, doc_id)
            .map_err(RemoteImportError::apply_failed)?
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>();
        let content_ops = reconcile::compute_reconcile_patch(&existing_entries, &entry.content)
            .map_err(RemoteImportError::apply_failed)?;
        Ok((
            Some(PreparedLedgerTarget::Upsert(PreparedLedgerUpsert {
                path: entry.path,
                doc_id,
                content_ops,
                inode: None,
            })),
            RemoteImportOverlapTarget {
                doc_id: Some(doc_id),
                ..overlap
            },
        ))
    }
}

fn validate_change_kind(
    entry: &RemoteImportPreparedEntry,
    current_content: Option<&str>,
) -> RemoteImportResult<()> {
    let current_digest = current_content.map(|content| RemoteImportDigest::of(content.as_bytes()));
    let valid = match entry.change_kind {
        RemoteImportChangeKind::Added => current_digest.is_none(),
        RemoteImportChangeKind::Modified => {
            current_digest.is_some_and(|digest| digest != entry.blob_digest)
        }
        RemoteImportChangeKind::Unchanged => current_digest == Some(entry.blob_digest),
    };
    if !valid {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "candidate change kind does not match authority at {:?}",
            entry.path
        )));
    }
    Ok(())
}
