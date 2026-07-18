//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 22_reliability_observability#observation-to-health-mapping

use crate::models::{DocId, RepoId};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub(super) const PROJECTION_FAULT_VALUE_VERSION: u16 = 1;
pub(super) const MAX_ERROR_CHARS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProjectionFaultKind {
    ProjectionWritebackFailed,
    ProjectionRebuildInterrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ProjectionFaultOrigin {
    ProjectionPersistence,
    ProjectionRepair,
    RemoteImport {
        session_id: u128,
        revision: u64,
        request_id: Uuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ProjectionFaultStatus {
    Pending,
}

pub(crate) struct ProjectionFaultInput<'a> {
    pub(crate) fault_kind: ProjectionFaultKind,
    pub(crate) target_path: Option<&'a str>,
    pub(crate) source_path: Option<&'a str>,
    pub(crate) doc_id: Option<DocId>,
    pub(crate) ledger_seq_or_head: Option<u64>,
    pub(crate) last_error: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DurableProjectionFault {
    pub(super) value_version: u16,
    pub(super) repo_id: RepoId,
    pub(super) repo_name_at_fault: String,
    pub(super) name_epoch: Option<u64>,
    pub(super) fault_kind: ProjectionFaultKind,
    pub(super) origin: ProjectionFaultOrigin,
    pub(super) target_path: Option<String>,
    pub(super) source_path: Option<String>,
    pub(super) doc_id: Option<DocId>,
    pub(super) ledger_seq_or_head: Option<u64>,
    pub(super) projection_workspace_root: Option<String>,
    pub(super) first_seen_at_unix_ms: i64,
    pub(super) last_seen_at_unix_ms: i64,
    pub(super) last_error: String,
    pub(super) retry_count: u32,
    pub(super) status: ProjectionFaultStatus,
}

pub(crate) struct PreparedProjectionFault {
    pub(super) key: [u8; 32],
    pub(super) value: DurableProjectionFault,
}

#[derive(Debug, Error)]
pub(crate) enum ProjectionFaultError {
    #[error("Projection Fault storage failure: {0}")]
    Storage(String),
    #[error("Projection Fault codec failure: {0}")]
    Codec(String),
    #[error("Projection Fault invariant failure: {0}")]
    Invariant(String),
}

impl ProjectionFaultError {
    pub(super) fn storage(error: impl std::fmt::Display) -> Self {
        Self::Storage(error.to_string())
    }

    pub(super) fn codec(error: impl std::fmt::Display) -> Self {
        Self::Codec(error.to_string())
    }
}

pub(crate) type ProjectionFaultResult<T> = Result<T, ProjectionFaultError>;
