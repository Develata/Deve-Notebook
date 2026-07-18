//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-session-contract
//!
//! Source-specific immutable input and workflow-table half of Remote Import Apply.
//! Ledger facts remain exclusively owned by the sealed writer.

mod transaction;

use super::artifact::VerifiedRemoteImportEntry;
use super::error::{RemoteImportError, RemoteImportResult};
use super::types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportBlocker, RemoteImportBranch,
    RemoteImportChangeKind, RemoteImportDigest, RemoteImportSessionRecord, RemoteImportState,
};
use crate::models::{GlobalSeq, PeerId, RepoId};

pub(crate) use transaction::{RemoteImportTxnAdmission, admit_in_txn, mark_applied_in_txn};

pub(crate) struct PreparedRemoteImportApply {
    context: RemoteImportApplyContext,
    entries: Vec<RemoteImportPreparedEntry>,
}

pub(crate) struct RemoteImportApplyContext {
    repo_id: RepoId,
    request: RemoteImportApplyRequest,
    writer_peer_id: PeerId,
    mode: RemoteImportApplyMode,
}

enum RemoteImportApplyMode {
    Fresh {
        expected: Box<RemoteImportSessionRecord>,
        evidence: Vec<RemoteImportEntryEvidence>,
    },
    Replay {
        receipt: RemoteImportApplyReceipt,
    },
}

struct RemoteImportEntryEvidence {
    path: String,
    blob_digest: RemoteImportDigest,
    size: u64,
    change_kind: RemoteImportChangeKind,
    blockers: Vec<RemoteImportBlocker>,
}

pub(crate) struct RemoteImportPreparedEntry {
    pub(crate) path: String,
    pub(crate) blob_digest: RemoteImportDigest,
    pub(crate) change_kind: RemoteImportChangeKind,
    pub(crate) blockers: Vec<RemoteImportBlocker>,
    pub(crate) content: String,
}

impl PreparedRemoteImportApply {
    pub(in crate::remote_import) fn fresh(
        record: RemoteImportSessionRecord,
        request: RemoteImportApplyRequest,
        writer_peer_id: PeerId,
        entries: Vec<VerifiedRemoteImportEntry>,
    ) -> RemoteImportResult<Self> {
        if record.state != RemoteImportState::Ready {
            return Err(invalid_state(&record, "Ready"));
        }
        let candidate = record.candidate.as_ref().ok_or_else(|| {
            RemoteImportError::ArtifactTampered("ready session candidate is missing".to_string())
        })?;
        if request.session_id != record.session_id || request.revision != candidate.revision {
            return Err(RemoteImportError::StaleGeneration(request.session_id));
        }
        if candidate.entry_count as usize != entries.len() {
            return Err(RemoteImportError::ArtifactTampered(
                "verified Apply entry count differs from candidate".to_string(),
            ));
        }
        let blockers = collect_blockers(entries.iter().flat_map(|entry| &entry.blockers));
        if !blockers.is_empty() {
            return Err(RemoteImportError::Blocked {
                session_id: record.session_id,
                blockers,
            });
        }
        let evidence = entries
            .iter()
            .map(|entry| RemoteImportEntryEvidence {
                path: entry.path.clone(),
                blob_digest: entry.blob_digest,
                size: entry.size,
                change_kind: entry.change_kind,
                blockers: entry.blockers.clone(),
            })
            .collect();
        let prepared_entries = entries
            .into_iter()
            .map(|entry| RemoteImportPreparedEntry {
                path: entry.path,
                blob_digest: entry.blob_digest,
                change_kind: entry.change_kind,
                blockers: entry.blockers,
                content: entry.content,
            })
            .collect();
        Ok(Self {
            context: RemoteImportApplyContext {
                repo_id: record.repo_id,
                request,
                writer_peer_id,
                mode: RemoteImportApplyMode::Fresh {
                    expected: Box::new(record),
                    evidence,
                },
            },
            entries: prepared_entries,
        })
    }

    pub(in crate::remote_import) fn replay(
        record: RemoteImportSessionRecord,
        request: RemoteImportApplyRequest,
        writer_peer_id: PeerId,
    ) -> RemoteImportResult<Self> {
        let receipt = validate_stored_replay_receipt(&record, &request, &writer_peer_id)?.clone();
        Ok(Self {
            context: RemoteImportApplyContext {
                repo_id: record.repo_id,
                request,
                writer_peer_id,
                mode: RemoteImportApplyMode::Replay { receipt },
            },
            entries: Vec::new(),
        })
    }

    pub(crate) fn into_parts(self) -> (RemoteImportApplyContext, Vec<RemoteImportPreparedEntry>) {
        (self.context, self.entries)
    }
}

fn validate_stored_replay_receipt<'a>(
    record: &'a RemoteImportSessionRecord,
    request: &RemoteImportApplyRequest,
    writer_peer_id: &PeerId,
) -> RemoteImportResult<&'a RemoteImportApplyReceipt> {
    if record.state != RemoteImportState::Applied {
        return Err(invalid_state(record, "Applied"));
    }
    if record.branch != RemoteImportBranch::Local {
        return Err(RemoteImportError::ArtifactTampered(
            "applied session is no longer bound to the local branch".to_string(),
        ));
    }
    let source = record.source_snapshot.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered(
            "applied session source snapshot is missing".to_string(),
        )
    })?;
    let candidate = record.candidate.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("applied session candidate is missing".to_string())
    })?;
    let receipt = record.apply_receipt.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("applied session receipt is missing".to_string())
    })?;
    if request.request_id != receipt.request_id
        || request.session_id != record.session_id
        || request.session_id != receipt.session_id
        || request.revision != candidate.revision
        || request.revision != receipt.revision
    {
        return Err(RemoteImportError::InvalidState {
            session_id: record.session_id,
            state: record.state,
            expected: "same stored Apply request/session/revision",
        });
    }
    if receipt.writer_peer_id != *writer_peer_id
        || receipt.authority_head_before != candidate.ledger_head
        || receipt.authority_head_after < receipt.authority_head_before
        || receipt.manifest_digest != source.manifest_digest
        || receipt.candidate_digest != candidate.candidate_digest
    {
        return Err(RemoteImportError::ArtifactTampered(
            "stored Apply receipt immutable core differs from sealed session".to_string(),
        ));
    }
    Ok(receipt)
}

fn same_receipt_immutable_core(
    left: &RemoteImportApplyReceipt,
    right: &RemoteImportApplyReceipt,
) -> bool {
    left.request_id == right.request_id
        && left.session_id == right.session_id
        && left.revision == right.revision
        && left.writer_peer_id == right.writer_peer_id
        && left.authority_head_before == right.authority_head_before
        && left.authority_head_after == right.authority_head_after
        && left.manifest_digest == right.manifest_digest
        && left.candidate_digest == right.candidate_digest
}

impl RemoteImportApplyContext {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn expected_head(&self) -> GlobalSeq {
        match &self.mode {
            RemoteImportApplyMode::Fresh { expected, .. } => {
                expected
                    .candidate
                    .as_ref()
                    .expect("fresh Remote Import context has a candidate")
                    .ledger_head
            }
            RemoteImportApplyMode::Replay { receipt } => receipt.authority_head_before,
        }
    }

    pub(crate) fn is_replay(&self) -> bool {
        matches!(self.mode, RemoteImportApplyMode::Replay { .. })
    }

    pub(crate) fn session_id(&self) -> super::types::RemoteImportSessionId {
        self.request.session_id
    }

    pub(crate) fn expected_mutation_count(&self) -> usize {
        match &self.mode {
            RemoteImportApplyMode::Fresh { evidence, .. } => evidence
                .iter()
                .filter(|entry| entry.change_kind != RemoteImportChangeKind::Unchanged)
                .count(),
            RemoteImportApplyMode::Replay { .. } => 0,
        }
    }

    pub(crate) fn pending_receipt(
        &self,
        authority_head_after: GlobalSeq,
    ) -> RemoteImportResult<RemoteImportApplyReceipt> {
        let RemoteImportApplyMode::Fresh { expected, .. } = &self.mode else {
            return Err(RemoteImportError::ApplyFailed(
                "replay context cannot create a second receipt".to_string(),
            ));
        };
        let source = expected.source_snapshot.as_ref().ok_or_else(|| {
            RemoteImportError::ArtifactTampered("ready source snapshot is missing".to_string())
        })?;
        let candidate = expected.candidate.as_ref().ok_or_else(|| {
            RemoteImportError::ArtifactTampered("ready candidate is missing".to_string())
        })?;
        Ok(RemoteImportApplyReceipt {
            request_id: self.request.request_id,
            session_id: expected.session_id,
            revision: candidate.revision,
            writer_peer_id: self.writer_peer_id.clone(),
            authority_head_before: candidate.ledger_head,
            authority_head_after,
            manifest_digest: source.manifest_digest,
            candidate_digest: candidate.candidate_digest,
            projection_outcome: super::types::RemoteImportProjectionOutcome::Pending,
        })
    }
}

fn collect_blockers<'a>(
    blockers: impl Iterator<Item = &'a RemoteImportBlocker>,
) -> Vec<RemoteImportBlocker> {
    let mut unique = Vec::new();
    for blocker in blockers {
        if !unique.contains(blocker) {
            unique.push(*blocker);
        }
    }
    unique
}

fn invalid_state(record: &RemoteImportSessionRecord, expected: &'static str) -> RemoteImportError {
    RemoteImportError::InvalidState {
        session_id: record.session_id,
        state: record.state,
        expected,
    }
}
