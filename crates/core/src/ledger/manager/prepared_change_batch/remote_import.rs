//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 06_backup#remote-import-session-contract
//!
//! Remote Import transaction half of the sealed writer.

use super::{
    PreparedLedgerChangeBatch, PreparedLedgerChangeSource, PreparedLedgerTarget,
    append_targets_to_txn, ledger_head_in_txn,
};
use crate::ledger::{RepoManager, ops};
use crate::models::{DocId, GlobalSeq};
use crate::remote_import::apply::{
    RemoteImportApplyContext, RemoteImportTxnAdmission, admit_in_txn, mark_applied_in_txn,
};
use crate::remote_import::{
    RemoteImportApplyReceipt, RemoteImportBlocker, RemoteImportError, RemoteImportResult,
};
use crate::source_control::pending_fs::PendingFsEntry;
use crate::source_control::staging::{STAGED_TABLE, StagedEntry};
use redb::ReadableTable;
use std::collections::BTreeSet;

const REMOTE_IMPORT_ACTOR: &str = "remote_import_apply";

pub(super) struct PreparedRemoteImportBatch {
    pub(super) context: RemoteImportApplyContext,
    pub(super) overlap_targets: Vec<RemoteImportOverlapTarget>,
}

pub(in crate::ledger::manager) struct RemoteImportOverlapTarget {
    pub(in crate::ledger::manager) path: String,
    pub(in crate::ledger::manager) doc_id: Option<DocId>,
}

pub(in crate::ledger::manager) fn commit_remote_import(
    manager: &RepoManager,
    repo_name: &str,
    batch: PreparedLedgerChangeBatch,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    let PreparedLedgerChangeBatch {
        repo_id,
        expected_ledger_head,
        targets,
        source,
    } = batch;
    let remote = match source {
        PreparedLedgerChangeSource::RemoteImport(remote) => remote,
        PreparedLedgerChangeSource::External(_) => {
            return Err(RemoteImportError::ApplyFailed(
                "External Apply sealed batch cannot enter Remote Import writer".to_string(),
            ));
        }
    };
    if repo_id != remote.context.repo_id()
        || expected_ledger_head != remote.context.expected_head().storage_key()
        || targets.len() != remote.context.expected_mutation_count()
    {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import sealed batch identity/count invariant failed".to_string(),
        ));
    }
    manager
        .run_on_local_repo(repo_name, |db| {
            Ok(commit_in_db(manager, db, repo_name, &targets, &remote))
        })
        .map_err(RemoteImportError::apply_failed)?
}

fn commit_in_db(
    manager: &RepoManager,
    db: &redb::Database,
    repo_name: &str,
    targets: &[PreparedLedgerTarget],
    remote: &PreparedRemoteImportBatch,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    let write_txn = db.begin_write().map_err(RemoteImportError::storage)?;
    let head_before = GlobalSeq::from_storage_key(
        ledger_head_in_txn(&write_txn).map_err(RemoteImportError::apply_failed)?,
    );
    match admit_in_txn(
        &write_txn,
        &remote.context,
        manager.local_peer_id(),
        head_before,
    )? {
        RemoteImportTxnAdmission::Replay(receipt) => return Ok(receipt),
        RemoteImportTxnAdmission::Stale(blockers) => {
            write_txn.commit().map_err(RemoteImportError::storage)?;
            return Err(RemoteImportError::Stale {
                session_id: remote.context.session_id(),
                blockers,
            });
        }
        RemoteImportTxnAdmission::Fresh => {}
    }
    let blockers = detect_overlap(&write_txn, &remote.overlap_targets)?;
    if !blockers.is_empty() {
        return Err(RemoteImportError::Blocked {
            session_id: remote.context.session_id(),
            blockers,
        });
    }
    append_targets_to_txn(
        &write_txn,
        manager.local_peer_id(),
        &ops::local_repo_scope(repo_name),
        REMOTE_IMPORT_ACTOR,
        targets,
    )
    .map_err(RemoteImportError::apply_failed)?;
    let head_after = GlobalSeq::from_storage_key(
        ledger_head_in_txn(&write_txn).map_err(RemoteImportError::apply_failed)?,
    );
    let receipt = remote.context.pending_receipt(head_after)?;
    mark_applied_in_txn(&write_txn, &remote.context, &receipt)?;
    write_txn.commit().map_err(RemoteImportError::storage)?;
    Ok(receipt)
}

fn detect_overlap(
    write_txn: &redb::WriteTransaction,
    targets: &[RemoteImportOverlapTarget],
) -> RemoteImportResult<Vec<RemoteImportBlocker>> {
    let paths = targets
        .iter()
        .map(|target| target.path.as_str())
        .collect::<BTreeSet<_>>();
    let docs = targets
        .iter()
        .filter_map(|target| target.doc_id.map(|doc_id| doc_id.as_u128()))
        .collect::<BTreeSet<_>>();
    let pending_overlap = {
        let pending = write_txn
            .open_table(crate::ledger::schema::PENDING_FS_OPS)
            .map_err(RemoteImportError::storage)?;
        let mut overlap = false;
        for row in pending.iter().map_err(RemoteImportError::storage)? {
            let (path, bytes) = row.map_err(RemoteImportError::storage)?;
            let entry: PendingFsEntry =
                serde_json::from_slice(bytes.value()).map_err(RemoteImportError::json)?;
            if paths.contains(path.value())
                || paths.contains(entry.path.as_str())
                || entry
                    .renamed_from
                    .as_deref()
                    .is_some_and(|path| paths.contains(path))
                || entry
                    .doc_id
                    .is_some_and(|doc_id| docs.contains(&doc_id.as_u128()))
            {
                overlap = true;
                break;
            }
        }
        overlap
    };
    let staged_overlap = {
        let staged = write_txn
            .open_table(STAGED_TABLE)
            .map_err(RemoteImportError::storage)?;
        let mut overlap = false;
        for row in staged.iter().map_err(RemoteImportError::storage)? {
            let (path, bytes) = row.map_err(RemoteImportError::storage)?;
            let entry: StagedEntry =
                serde_json::from_slice(bytes.value()).map_err(RemoteImportError::json)?;
            if paths.contains(path.value())
                || entry
                    .renamed_from
                    .as_deref()
                    .is_some_and(|path| paths.contains(path))
                || entry
                    .doc_id
                    .is_some_and(|doc_id| docs.contains(&doc_id.as_u128()))
            {
                overlap = true;
                break;
            }
        }
        overlap
    };
    let mut blockers = Vec::new();
    if pending_overlap {
        blockers.push(RemoteImportBlocker::PendingOverlap);
    }
    if staged_overlap {
        blockers.push(RemoteImportBlocker::StagedOverlap);
    }
    Ok(blockers)
}
