//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-session-contract
//!
//! Safe host-facing Remote Import identities and view projections.

use super::super::error::{RemoteImportError, RemoteImportResult};
use super::super::runtime::RemoteImportCapture;
use super::super::types::{
    RemoteImportApplyReceipt, RemoteImportCandidateEntry, RemoteImportDigest,
    RemoteImportSessionRecord,
};
use super::super::{
    RemoteImportBlocker, RemoteImportCandidateRevision, RemoteImportChangeKind,
    RemoteImportProjectionOutcome, RemoteImportSessionId, RemoteImportState,
};
use crate::models::RepoId;
use crate::source_control::diff_projection::DiffProjection;
use crate::utils::path::to_forward_slash;
use sha2::{Digest, Sha256};
use std::io::Read;
use uuid::Uuid;

pub const REMOTE_IMPORT_DEFAULT_PAGE_SIZE: usize = 100;
pub const REMOTE_IMPORT_MAX_PAGE_SIZE: usize = 200;

#[derive(Clone, PartialEq, Eq)]
pub struct RemoteImportBinding(RemoteImportDigest);

impl RemoteImportBinding {
    /// Hashes a canonical, credential-free identity. `domain` must describe
    /// whether the material is a source identity or locator/profile binding.
    pub fn from_canonical_identity(domain: &str, identity: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"deve-remote-import-binding-v1\0");
        hasher.update((domain.len() as u64).to_le_bytes());
        hasher.update(domain.as_bytes());
        hasher.update((identity.len() as u64).to_le_bytes());
        hasher.update(identity);
        Self(RemoteImportDigest::from_bytes(hasher.finalize().into()))
    }

    pub(super) fn digest(&self) -> RemoteImportDigest {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportEntryId(String);

impl RemoteImportEntryId {
    pub fn parse(value: impl Into<String>) -> RemoteImportResult<Self> {
        let value = value.into();
        let valid = value.len() == 64
            && value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if !valid {
            return Err(RemoteImportError::ArtifactTampered(
                "Remote Import entry identity is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn from_digest(digest: RemoteImportDigest) -> Self {
        Self(digest.to_hex())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportPageCursor(String);

impl RemoteImportPageCursor {
    pub fn parse(value: impl Into<String>) -> RemoteImportResult<Self> {
        let value = value.into();
        let valid = value.len() == 64
            && value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if !valid {
            return Err(RemoteImportError::ArtifactTampered(
                "Remote Import page cursor is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportSessionView {
    pub session_id: RemoteImportSessionId,
    pub repo_id: RepoId,
    pub state: RemoteImportState,
    pub revision: Option<RemoteImportCandidateRevision>,
    pub entry_count: u32,
    pub blockers: Vec<RemoteImportBlocker>,
    pub cleanup_pending: bool,
    pub projection_outcome: Option<RemoteImportProjectionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportCandidateView {
    pub entry_id: RemoteImportEntryId,
    pub display_label: String,
    pub change_kind: RemoteImportChangeKind,
    pub blockers: Vec<RemoteImportBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportCandidatePage {
    pub session: RemoteImportSessionView,
    pub entries: Vec<RemoteImportCandidateView>,
    pub next_cursor: Option<RemoteImportPageCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportDiffView {
    pub entry_id: RemoteImportEntryId,
    pub display_label: String,
    pub change_kind: RemoteImportChangeKind,
    pub blockers: Vec<RemoteImportBlocker>,
    pub projection: DiffProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportApplyView {
    pub request_id: Uuid,
    pub session_id: RemoteImportSessionId,
    pub revision: RemoteImportCandidateRevision,
    pub projection_outcome: RemoteImportProjectionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportRepairPlan {
    pub finding_count: usize,
    pub repairable_count: usize,
    pub(super) token: String,
}

impl RemoteImportRepairPlan {
    pub fn token(&self) -> &str {
        &self.token
    }
}

pub struct RemoteImportCaptureSink {
    pub(super) inner: RemoteImportCapture,
}

impl RemoteImportCaptureSink {
    pub fn session_id(&self) -> RemoteImportSessionId {
        self.inner.session_id()
    }

    pub fn capture_file(&mut self, path: &str, reader: impl Read) -> RemoteImportResult<()> {
        self.inner.capture_file(path, reader)
    }

    pub fn finish(self) -> RemoteImportResult<RemoteImportSessionView> {
        self.inner
            .finish()
            .map(|record| session_view(&record, Vec::new()))
    }

    pub fn abort_source(self) -> RemoteImportResult<RemoteImportSessionView> {
        self.inner
            .abort_source()
            .map(|record| session_view(&record, Vec::new()))
    }
}

pub(super) fn session_view(
    record: &RemoteImportSessionRecord,
    blockers: Vec<RemoteImportBlocker>,
) -> RemoteImportSessionView {
    let state = if !blockers.is_empty()
        && matches!(
            record.state,
            RemoteImportState::Ready | RemoteImportState::Stale
        ) {
        RemoteImportState::Stale
    } else {
        record.state
    };
    RemoteImportSessionView {
        session_id: record.session_id,
        repo_id: record.repo_id,
        state,
        revision: record
            .candidate
            .as_ref()
            .map(|candidate| candidate.revision),
        entry_count: record
            .candidate
            .as_ref()
            .map(|candidate| candidate.entry_count)
            .unwrap_or(0),
        blockers,
        cleanup_pending: record.cleanup_pending,
        projection_outcome: record
            .apply_receipt
            .as_ref()
            .map(|receipt| receipt.projection_outcome),
    }
}

pub(super) fn candidate_view(entry: &RemoteImportCandidateEntry) -> RemoteImportCandidateView {
    RemoteImportCandidateView {
        entry_id: RemoteImportEntryId::from_digest(entry.entry_id),
        display_label: display_label(&entry.path),
        change_kind: entry.change_kind,
        blockers: entry.blockers.clone(),
    }
}

pub(super) fn display_label(path: &str) -> String {
    to_forward_slash(path)
}

pub(super) fn cursor_start(
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
    entries: &[RemoteImportCandidateEntry],
    cursor: Option<&RemoteImportPageCursor>,
) -> RemoteImportResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    entries
        .iter()
        .position(|entry| {
            page_cursor(session_id, revision, entry.entry_id).as_str() == cursor.as_str()
        })
        .map(|index| index + 1)
        .ok_or_else(|| {
            RemoteImportError::ArtifactTampered(
                "Remote Import page cursor does not belong to this revision".to_string(),
            )
        })
}

pub(super) fn page_cursor(
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
    entry_id: RemoteImportDigest,
) -> RemoteImportPageCursor {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-remote-import-page-cursor-v1\0");
    hasher.update(session_id.as_uuid().as_bytes());
    hasher.update(revision.get().to_le_bytes());
    hasher.update(entry_id.as_bytes());
    RemoteImportPageCursor(hex::encode(hasher.finalize()))
}

pub(super) fn apply_view(receipt: &RemoteImportApplyReceipt) -> RemoteImportApplyView {
    RemoteImportApplyView {
        request_id: receipt.request_id,
        session_id: receipt.session_id,
        revision: receipt.revision,
        projection_outcome: receipt.projection_outcome,
    }
}
