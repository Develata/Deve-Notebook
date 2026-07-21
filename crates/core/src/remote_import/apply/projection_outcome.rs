//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 03_storage/authority#projection-fault-recovery-table
//!   - 03_storage/projection#remote-import-projection-writeback
//!
//! Remote Import owns the receipt CAS. Projection persistence contributes only a prepared,
//! same-transaction fault upsert; neither side receives a generic transaction callback.

use super::{same_receipt_immutable_core, validate_stored_replay_receipt};
use crate::ledger::RepoManager;
use crate::projection_fault::{
    PreparedProjectionFault, prepare_remote_import_fault, record_prepared_in_txn,
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::store::{RemoteImportStore, decode_session, encode};
use crate::remote_import::types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportProjectionOutcome,
};
use redb::ReadableTable;

pub(in crate::remote_import) fn settle_projection_written(
    store: &RemoteImportStore,
    expected: &RemoteImportApplyReceipt,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    settle(store, expected, ProjectionSettlement::Written)
}

pub(in crate::remote_import) fn settle_projection_degraded(
    store: &RemoteImportStore,
    expected: &RemoteImportApplyReceipt,
    last_error: &str,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    let info = store
        .with_db(|db| {
            RepoManager::read_local_repo_info_from_db(db).map_err(RemoteImportError::storage)
        })?
        .ok_or_else(|| {
            RemoteImportError::Storage(
                "Remote Import projection settlement RepoInfo is missing".to_string(),
            )
        })?;
    if info.uuid != store.repo_id() {
        return Err(RemoteImportError::Storage(
            "Remote Import projection settlement RepoId differs from local authority".to_string(),
        ));
    }
    let fault = prepare_remote_import_fault(
        info.uuid,
        info.name,
        expected.session_id.as_u128(),
        expected.revision.get(),
        expected.request_id,
        expected.authority_head_after.storage_key(),
        last_error,
    );
    settle(store, expected, ProjectionSettlement::Degraded(&fault))
}

enum ProjectionSettlement<'a> {
    Written,
    Degraded(&'a PreparedProjectionFault),
}

impl ProjectionSettlement<'_> {
    fn outcome(&self) -> RemoteImportProjectionOutcome {
        match self {
            Self::Written => RemoteImportProjectionOutcome::Written,
            Self::Degraded(_) => RemoteImportProjectionOutcome::Degraded,
        }
    }
}

fn settle(
    store: &RemoteImportStore,
    expected: &RemoteImportApplyReceipt,
    settlement: ProjectionSettlement<'_>,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    let db = store.lease_db()?;
    let write = db.begin_write().map_err(RemoteImportError::storage)?;
    let receipt = settle_in_txn(&write, store, expected, settlement)?;
    write.commit().map_err(RemoteImportError::storage)?;
    Ok(receipt)
}

fn settle_in_txn(
    write: &redb::WriteTransaction,
    store: &RemoteImportStore,
    expected: &RemoteImportApplyReceipt,
    settlement: ProjectionSettlement<'_>,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    super::transaction::verify_repo_schema_and_identity(
        write,
        store.repo_id(),
        expected.session_id,
    )?;
    let mut sessions = write
        .open_table(crate::ledger::schema::REMOTE_IMPORT_SESSIONS)
        .map_err(RemoteImportError::storage)?;
    let guard = sessions
        .get(&expected.session_id.as_u128())
        .map_err(RemoteImportError::storage)?
        .ok_or(RemoteImportError::SessionNotFound(expected.session_id))?;
    let mut record = decode_session(
        expected.session_id.as_u128(),
        guard.value(),
        store.repo_id(),
    )?;
    drop(guard);

    let candidate = record.candidate.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered(
            "applied session candidate is missing during projection settlement".to_string(),
        )
    })?;
    let replay_request = RemoteImportApplyRequest {
        request_id: expected.request_id,
        session_id: expected.session_id,
        revision: expected.revision,
        locator_digest: candidate.locator_digest,
        ignore_digest: candidate.ignore_digest,
    };
    let stored =
        validate_stored_replay_receipt(&record, &replay_request, &expected.writer_peer_id)?;
    if !same_receipt_immutable_core(stored, expected) {
        return Err(RemoteImportError::ArtifactTampered(
            "projection settlement receipt immutable core differs from stored Apply".to_string(),
        ));
    }

    let target = settlement.outcome();
    if stored.projection_outcome == target {
        return Ok(stored.clone());
    }
    if stored.projection_outcome != RemoteImportProjectionOutcome::Pending {
        return Err(RemoteImportError::ApplyFailed(
            "projection receipt terminal outcome cannot be changed".to_string(),
        ));
    }

    if let ProjectionSettlement::Degraded(fault) = settlement {
        record_prepared_in_txn(write, store.repo_id(), fault)
            .map_err(RemoteImportError::apply_failed)?;
    }

    let mut settled = stored.clone();
    settled.projection_outcome = target;
    record.apply_receipt = Some(settled.clone());
    let bytes = encode(&record)?;
    sessions
        .insert(&record.session_id.as_u128(), bytes.as_slice())
        .map_err(RemoteImportError::storage)?;
    Ok(settled)
}

#[cfg(test)]
pub(in crate::remote_import) fn settle_degraded_without_commit_for_test(
    store: &RemoteImportStore,
    expected: &RemoteImportApplyReceipt,
    fault: &PreparedProjectionFault,
) -> RemoteImportResult<RemoteImportApplyReceipt> {
    let db = store.lease_db()?;
    let write = db.begin_write().map_err(RemoteImportError::storage)?;
    let receipt = settle_in_txn(
        &write,
        store,
        expected,
        ProjectionSettlement::Degraded(fault),
    )?;
    drop(write);
    Ok(receipt)
}
