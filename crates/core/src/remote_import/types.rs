//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-state-machine

use crate::models::{GlobalSeq, PeerId, RepoId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

pub(super) const REMOTE_IMPORT_VALUE_VERSION: u16 = 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct RemoteImportDigest([u8; 32]);

impl RemoteImportDigest {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(crate::security::hashing::sha256_bytes(bytes))
    }

    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for RemoteImportDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RemoteImportDigest")
            .field(&self.to_hex())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct RemoteImportSessionId(Uuid);

impl RemoteImportSessionId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub(super) fn from_uuid_for_artifact(value: Uuid) -> Self {
        Self(value)
    }

    pub(crate) fn as_u128(self) -> u128 {
        self.0.as_u128()
    }
}

impl fmt::Display for RemoteImportSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct RemoteImportCandidateRevision(u64);

impl RemoteImportCandidateRevision {
    pub(crate) const FIRST: Self = Self(1);

    pub(crate) fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    #[cfg(test)]
    pub(super) const fn from_u64_for_test(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportBranch {
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportState {
    Preparing,
    Ready,
    Stale,
    Failed,
    Applied,
    Discarded,
}

impl RemoteImportState {
    pub(super) fn is_terminal(self) -> bool {
        matches!(self, Self::Applied | Self::Discarded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportChangeKind {
    Added,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportBlocker {
    LedgerHeadDrift,
    IgnoreSnapshotDrift,
    LocatorBindingDrift,
    PendingOverlap,
    StagedOverlap,
    ArtifactTamper,
    RepoMembershipMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportFailurePhase {
    Reserve,
    Capture,
    Publish,
    Recovery,
    Verify,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportFailureKind {
    Interrupted,
    InvalidPath,
    LimitExceeded,
    SourceRead,
    ArtifactIo,
    DigestMismatch,
    InvalidState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportFailure {
    pub(crate) phase: RemoteImportFailurePhase,
    pub(crate) kind: RemoteImportFailureKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportSourceSnapshot {
    pub(crate) manifest_digest: RemoteImportDigest,
    pub(crate) blob_set_digest: RemoteImportDigest,
    pub(crate) file_count: u32,
    pub(crate) payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportCandidateEntry {
    pub(crate) entry_id: RemoteImportDigest,
    pub(crate) path: String,
    pub(crate) blob_digest: RemoteImportDigest,
    pub(crate) size: u64,
    pub(crate) change_kind: RemoteImportChangeKind,
    pub(crate) blockers: Vec<RemoteImportBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportCandidateRevisionRecord {
    pub(crate) revision: RemoteImportCandidateRevision,
    pub(crate) candidate_digest: RemoteImportDigest,
    pub(crate) ledger_head: GlobalSeq,
    pub(crate) ignore_digest: RemoteImportDigest,
    pub(crate) locator_digest: RemoteImportDigest,
    pub(crate) entry_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RemoteImportProjectionOutcome {
    Pending,
    Written,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportApplyReceipt {
    pub(crate) request_id: Uuid,
    pub(crate) session_id: RemoteImportSessionId,
    pub(crate) revision: RemoteImportCandidateRevision,
    pub(crate) writer_peer_id: PeerId,
    pub(crate) authority_head_before: GlobalSeq,
    pub(crate) authority_head_after: GlobalSeq,
    pub(crate) manifest_digest: RemoteImportDigest,
    pub(crate) candidate_digest: RemoteImportDigest,
    pub(crate) projection_outcome: RemoteImportProjectionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteImportSessionRecord {
    pub(super) value_version: u16,
    pub(crate) session_id: RemoteImportSessionId,
    pub(crate) repo_id: RepoId,
    pub(crate) branch: RemoteImportBranch,
    pub(crate) generation: u64,
    pub(crate) order: u64,
    pub(crate) state: RemoteImportState,
    pub(crate) source_binding_digest: RemoteImportDigest,
    pub(crate) locator_binding_digest: RemoteImportDigest,
    pub(crate) baseline_head: GlobalSeq,
    pub(crate) ignore_digest: RemoteImportDigest,
    pub(crate) source_snapshot: Option<RemoteImportSourceSnapshot>,
    pub(crate) candidate: Option<RemoteImportCandidateRevisionRecord>,
    pub(crate) failure: Option<RemoteImportFailure>,
    pub(crate) apply_receipt: Option<RemoteImportApplyReceipt>,
    pub(crate) cleanup_pending: bool,
}

impl RemoteImportSessionRecord {
    pub(super) fn validate_value_version(&self) -> bool {
        self.value_version == REMOTE_IMPORT_VALUE_VERSION
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RemoteImportRuntimeRecord {
    pub(super) value_version: u16,
    pub(super) active_session: Option<RemoteImportSessionId>,
    pub(super) next_generation: u64,
    pub(super) next_order: u64,
}

impl Default for RemoteImportRuntimeRecord {
    fn default() -> Self {
        Self {
            value_version: REMOTE_IMPORT_VALUE_VERSION,
            active_session: None,
            next_generation: 1,
            next_order: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImportBaseline {
    pub(crate) ledger_head: GlobalSeq,
    pub(crate) ignore_digest: RemoteImportDigest,
    pub(crate) locator_digest: RemoteImportDigest,
    pub(crate) existing: BTreeMap<String, RemoteImportDigest>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImportPrepareRequest {
    pub(crate) source_binding_digest: RemoteImportDigest,
    pub(crate) locator_binding_digest: RemoteImportDigest,
    pub(crate) baseline: RemoteImportBaseline,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImportRefreshRequest {
    pub(crate) source_binding_digest: RemoteImportDigest,
    pub(crate) baseline: RemoteImportBaseline,
}
