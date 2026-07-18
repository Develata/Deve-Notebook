//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 03_storage/authority#remote-import-workflow-tables
//!
//! Exact Redb admission and workflow-state transition for a sealed Apply.

use super::{
    RemoteImportApplyContext, RemoteImportApplyMode, RemoteImportEntryEvidence, collect_blockers,
    same_receipt_immutable_core, validate_stored_replay_receipt,
};
use crate::ledger::RepoInfo;
use crate::ledger::schema::{
    REDB_SCHEMA_VERSION, REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS, REPO_INFO_METADATA_KEY,
    REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use crate::models::{GlobalSeq, PeerId, RepoId};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::store::{RUNTIME_KEY, decode_runtime, decode_session, encode};
use crate::remote_import::types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportBlocker, RemoteImportBranch,
    RemoteImportDigest, RemoteImportRuntimeRecord, RemoteImportSessionRecord, RemoteImportState,
};
use redb::ReadableTable;
use sha2::{Digest, Sha256};

pub(crate) enum RemoteImportTxnAdmission {
    Fresh,
    Replay(RemoteImportApplyReceipt),
    Stale(Vec<RemoteImportBlocker>),
}

pub(crate) fn admit_in_txn(
    write_txn: &redb::WriteTransaction,
    context: &RemoteImportApplyContext,
    actual_writer: &PeerId,
    actual_head: GlobalSeq,
) -> RemoteImportResult<RemoteImportTxnAdmission> {
    verify_repo_schema_and_identity(write_txn, context.repo_id, context.request.session_id)?;
    if actual_writer != &context.writer_peer_id {
        return Err(RemoteImportError::ApplyFailed(
            "writer identity changed after Remote Import preflight".to_string(),
        ));
    }
    let (record, runtime) = read_workflow_rows(write_txn, context)?;
    match &context.mode {
        RemoteImportApplyMode::Replay { receipt } => {
            let current = validate_stored_replay_receipt(&record, &context.request, actual_writer)?;
            if !same_receipt_immutable_core(current, receipt) {
                return Err(RemoteImportError::ArtifactTampered(
                    "stored Apply receipt immutable core changed after replay preflight"
                        .to_string(),
                ));
            }
            // cleanup debt, a newer active session, and projection outcome are orthogonal to the
            // already-committed authority core. Return the latest durable outcome without append.
            Ok(RemoteImportTxnAdmission::Replay(current.clone()))
        }
        RemoteImportApplyMode::Fresh { expected, evidence } => {
            if &record != expected.as_ref()
                || record.state != RemoteImportState::Ready
                || runtime.active_session != Some(record.session_id)
                || record.branch != RemoteImportBranch::Local
            {
                return Err(RemoteImportError::StaleGeneration(record.session_id));
            }
            verify_fresh_evidence(&record, &context.request, evidence)?;
            let candidate = record.candidate.as_ref().expect("validated candidate");
            let mut stale = Vec::new();
            if actual_head != candidate.ledger_head {
                stale.push(RemoteImportBlocker::LedgerHeadDrift);
            }
            if context.request.locator_digest != candidate.locator_digest {
                stale.push(RemoteImportBlocker::LocatorBindingDrift);
            }
            if context.request.ignore_digest != candidate.ignore_digest {
                stale.push(RemoteImportBlocker::IgnoreSnapshotDrift);
            }
            if !stale.is_empty() {
                persist_stale_in_txn(write_txn, record)?;
                return Ok(RemoteImportTxnAdmission::Stale(stale));
            }
            let blockers = collect_blockers(evidence.iter().flat_map(|entry| &entry.blockers));
            if !blockers.is_empty() {
                return Err(RemoteImportError::Blocked {
                    session_id: record.session_id,
                    blockers,
                });
            }
            Ok(RemoteImportTxnAdmission::Fresh)
        }
    }
}

pub(crate) fn mark_applied_in_txn(
    write_txn: &redb::WriteTransaction,
    context: &RemoteImportApplyContext,
    receipt: &RemoteImportApplyReceipt,
) -> RemoteImportResult<()> {
    let RemoteImportApplyMode::Fresh { expected, .. } = &context.mode else {
        return Err(RemoteImportError::ApplyFailed(
            "replay context cannot mark a second Applied transition".to_string(),
        ));
    };
    validate_receipt(context, expected.as_ref(), receipt)?;
    let mut sessions = write_txn
        .open_table(REMOTE_IMPORT_SESSIONS)
        .map_err(RemoteImportError::storage)?;
    let guard = sessions
        .get(&expected.session_id.as_u128())
        .map_err(RemoteImportError::storage)?
        .ok_or(RemoteImportError::SessionNotFound(expected.session_id))?;
    let mut current = decode_session(
        expected.session_id.as_u128(),
        guard.value(),
        context.repo_id,
    )?;
    drop(guard);
    if &current != expected.as_ref() || current.state != RemoteImportState::Ready {
        return Err(RemoteImportError::StaleGeneration(expected.session_id));
    }
    current.state = RemoteImportState::Applied;
    current.apply_receipt = Some(receipt.clone());
    current.cleanup_pending = true;
    let bytes = encode(&current)?;
    sessions
        .insert(&current.session_id.as_u128(), bytes.as_slice())
        .map_err(RemoteImportError::storage)?;
    drop(sessions);

    let mut runtime_table = write_txn
        .open_table(REMOTE_IMPORT_RUNTIME)
        .map_err(RemoteImportError::storage)?;
    let guard = runtime_table
        .get(&RUNTIME_KEY)
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("runtime row missing".to_string()))?;
    let mut runtime = decode_runtime(guard.value())?;
    drop(guard);
    if runtime.active_session != Some(expected.session_id) {
        return Err(RemoteImportError::StaleGeneration(expected.session_id));
    }
    runtime.active_session = None;
    let bytes = encode(&runtime)?;
    runtime_table
        .insert(&RUNTIME_KEY, bytes.as_slice())
        .map_err(RemoteImportError::storage)?;
    Ok(())
}

fn verify_repo_schema_and_identity(
    write_txn: &redb::WriteTransaction,
    expected_repo_id: RepoId,
    session_id: crate::remote_import::types::RemoteImportSessionId,
) -> RemoteImportResult<()> {
    let metadata = write_txn
        .open_table(REPO_METADATA)
        .map_err(RemoteImportError::storage)?;
    let version = metadata
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("repo schema version missing".to_string()))?;
    let version: u16 = crate::codec::decode(version.value()).map_err(RemoteImportError::codec)?;
    if version != REDB_SCHEMA_VERSION {
        return Err(RemoteImportError::Storage(format!(
            "Remote Import expected redb schema v{REDB_SCHEMA_VERSION}, observed v{version}"
        )));
    }
    let info = metadata
        .get(&REPO_INFO_METADATA_KEY)
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("local RepoInfo missing".to_string()))?;
    let info: RepoInfo = crate::codec::decode(info.value()).map_err(RemoteImportError::codec)?;
    if info.uuid != expected_repo_id {
        return Err(RemoteImportError::Blocked {
            session_id,
            blockers: vec![RemoteImportBlocker::RepoMembershipMismatch],
        });
    }
    Ok(())
}

fn read_workflow_rows(
    write_txn: &redb::WriteTransaction,
    context: &RemoteImportApplyContext,
) -> RemoteImportResult<(RemoteImportSessionRecord, RemoteImportRuntimeRecord)> {
    let sessions = write_txn
        .open_table(REMOTE_IMPORT_SESSIONS)
        .map_err(RemoteImportError::storage)?;
    let guard = sessions
        .get(&context.request.session_id.as_u128())
        .map_err(RemoteImportError::storage)?
        .ok_or(RemoteImportError::SessionNotFound(
            context.request.session_id,
        ))?;
    let record = decode_session(
        context.request.session_id.as_u128(),
        guard.value(),
        context.repo_id,
    )?;
    drop(guard);
    drop(sessions);
    let runtime_table = write_txn
        .open_table(REMOTE_IMPORT_RUNTIME)
        .map_err(RemoteImportError::storage)?;
    let runtime = runtime_table
        .get(&RUNTIME_KEY)
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| RemoteImportError::Storage("runtime row missing".to_string()))?;
    let runtime = decode_runtime(runtime.value())?;
    Ok((record, runtime))
}

fn verify_fresh_evidence(
    record: &RemoteImportSessionRecord,
    request: &RemoteImportApplyRequest,
    evidence: &[RemoteImportEntryEvidence],
) -> RemoteImportResult<()> {
    let source = record.source_snapshot.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("ready source snapshot is missing".to_string())
    })?;
    let candidate = record.candidate.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("ready candidate is missing".to_string())
    })?;
    if request.session_id != record.session_id
        || request.revision != candidate.revision
        || candidate.entry_count as usize != evidence.len()
        || source.file_count as usize != evidence.len()
    {
        return Err(RemoteImportError::StaleGeneration(request.session_id));
    }
    let payload_bytes = evidence.iter().try_fold(0u64, |total, entry| {
        total.checked_add(entry.size).ok_or_else(|| {
            RemoteImportError::ArtifactTampered("Apply evidence size overflow".to_string())
        })
    })?;
    if payload_bytes != source.payload_bytes || digest_evidence(evidence) != source.blob_set_digest
    {
        return Err(RemoteImportError::ArtifactTampered(
            "Apply blob evidence differs from sealed source snapshot".to_string(),
        ));
    }
    if !evidence.windows(2).all(|pair| pair[0].path < pair[1].path) {
        return Err(RemoteImportError::ArtifactTampered(
            "Apply evidence paths are not strictly sorted".to_string(),
        ));
    }
    Ok(())
}

fn digest_evidence(entries: &[RemoteImportEntryEvidence]) -> RemoteImportDigest {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-remote-import-blob-set-v1\0");
    for entry in entries {
        hasher.update((entry.path.len() as u64).to_be_bytes());
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.size.to_be_bytes());
        hasher.update(entry.blob_digest.as_bytes());
    }
    RemoteImportDigest::from_bytes(hasher.finalize().into())
}

fn persist_stale_in_txn(
    write_txn: &redb::WriteTransaction,
    mut record: RemoteImportSessionRecord,
) -> RemoteImportResult<()> {
    record.state = RemoteImportState::Stale;
    let bytes = encode(&record)?;
    write_txn
        .open_table(REMOTE_IMPORT_SESSIONS)
        .map_err(RemoteImportError::storage)?
        .insert(&record.session_id.as_u128(), bytes.as_slice())
        .map_err(RemoteImportError::storage)?;
    Ok(())
}

fn validate_receipt(
    context: &RemoteImportApplyContext,
    expected: &RemoteImportSessionRecord,
    receipt: &RemoteImportApplyReceipt,
) -> RemoteImportResult<()> {
    let source = expected.source_snapshot.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("ready source snapshot is missing".to_string())
    })?;
    let candidate = expected.candidate.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("ready candidate is missing".to_string())
    })?;
    if receipt.request_id != context.request.request_id
        || receipt.session_id != expected.session_id
        || receipt.revision != candidate.revision
        || receipt.writer_peer_id != context.writer_peer_id
        || receipt.authority_head_before != candidate.ledger_head
        || receipt.authority_head_after < receipt.authority_head_before
        || receipt.manifest_digest != source.manifest_digest
        || receipt.candidate_digest != candidate.candidate_digest
        || receipt.projection_outcome
            != crate::remote_import::types::RemoteImportProjectionOutcome::Pending
    {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import receipt does not match sealed Apply context".to_string(),
        ));
    }
    Ok(())
}
