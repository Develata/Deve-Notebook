//! plan_ref:
//!   - 03_storage/projection#remote-import-projection-writeback
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 06_backup#remote-import-session-contract
//!
//! Post-commit projection settlement and Pending receipt recovery. Once the
//! Ledger transaction commits, every later failure is represented by the
//! durable receipt/fault state; it is never reported as an uncommitted Apply.

use super::super::artifact::{VerifiedRemoteImportEntry, verify_review_artifacts};
use super::super::runtime::RemoteImportRuntime;
use super::super::types::{
    RemoteImportApplyReceipt, RemoteImportCandidateEntry, RemoteImportChangeKind,
    RemoteImportProjectionOutcome, RemoteImportSessionRecord,
};
use crate::sync::SyncManager;

pub(super) fn writeback_paths(entries: &[VerifiedRemoteImportEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.change_kind != RemoteImportChangeKind::Unchanged)
        .map(|entry| entry.path.clone())
        .collect()
}

pub(super) fn settle_pending_projection(
    runtime: &RemoteImportRuntime,
    sync: &SyncManager,
    repo_name: &str,
    record: &RemoteImportSessionRecord,
    pending: &RemoteImportApplyReceipt,
) -> RemoteImportApplyReceipt {
    match verify_review_artifacts(&runtime.artifacts, record) {
        Ok(entries) => settle_pending_projection_with_paths(
            runtime,
            sync,
            repo_name,
            pending,
            &candidate_writeback_paths(&entries),
        ),
        Err(error) => finish_settlement(
            sync,
            repo_name,
            pending,
            runtime.settle_projection_degraded(pending, &error.to_string()),
        ),
    }
}

fn candidate_writeback_paths(entries: &[RemoteImportCandidateEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.change_kind != RemoteImportChangeKind::Unchanged)
        .map(|entry| entry.path.clone())
        .collect()
}

pub(super) fn settle_pending_projection_with_paths(
    runtime: &RemoteImportRuntime,
    sync: &SyncManager,
    repo_name: &str,
    pending: &RemoteImportApplyReceipt,
    paths: &[String],
) -> RemoteImportApplyReceipt {
    let settlement = match sync.writeback_remote_import_projection(
        repo_name,
        pending.authority_head_after,
        paths,
    ) {
        Ok(()) => runtime.settle_projection_written(pending),
        Err(error) => runtime.settle_projection_degraded(pending, &error.to_string()),
    };
    finish_settlement(sync, repo_name, pending, settlement)
}

fn finish_settlement(
    sync: &SyncManager,
    repo_name: &str,
    pending: &RemoteImportApplyReceipt,
    settlement: crate::remote_import::RemoteImportResult<RemoteImportApplyReceipt>,
) -> RemoteImportApplyReceipt {
    match settlement {
        Ok(receipt) => {
            match receipt.projection_outcome {
                RemoteImportProjectionOutcome::Written => {
                    if let Err(error) = sync.reconcile_remote_import_projection_health(repo_name) {
                        tracing::error!(
                            repo_name,
                            %error,
                            "Remote Import projection was written but health could not be proven; keeping repo degraded"
                        );
                        sync.mark_remote_import_projection_degraded(repo_name);
                    }
                }
                RemoteImportProjectionOutcome::Degraded
                | RemoteImportProjectionOutcome::Pending => {
                    sync.mark_remote_import_projection_degraded(repo_name);
                }
            }
            receipt
        }
        Err(error) => {
            tracing::error!(
                repo_name,
                session_id = %pending.session_id,
                request_id = %pending.request_id,
                %error,
                "Remote Import Ledger commit succeeded but projection settlement remains Pending"
            );
            sync.mark_remote_import_projection_degraded(repo_name);
            pending.clone()
        }
    }
}

#[cfg(test)]
pub(super) fn finish_settlement_for_test(
    sync: &SyncManager,
    repo_name: &str,
    pending: &RemoteImportApplyReceipt,
    settlement: crate::remote_import::RemoteImportResult<RemoteImportApplyReceipt>,
) -> RemoteImportApplyReceipt {
    finish_settlement(sync, repo_name, pending, settlement)
}
