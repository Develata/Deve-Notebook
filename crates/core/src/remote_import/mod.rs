//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 03_storage/authority#remote-import-workflow-tables
//!
//! Backend-only immutable Remote Import session runtime.
//!
//! This module owns host artifacts and workflow state only. It deliberately has no provider,
//! protocol, workspace, External Changes, or Source Control dependency. It does not own or
//! directly access Ledger fact tables; Apply crosses only the sealed authority writer API.

pub(crate) mod apply;
mod artifact;
mod error;
mod manifest;
mod repair;
mod runtime;
mod store;
mod types;

#[allow(unused_imports)]
// B1 defines the backend facade; B2-B4 add the first production consumers.
pub(crate) use error::{RemoteImportError, RemoteImportResult};
#[allow(unused_imports)]
pub(crate) use repair::{RemoteImportRepairFinding, RemoteImportRepairReport};
#[allow(unused_imports)]
pub(crate) use runtime::{RemoteImportCapture, RemoteImportRuntime, pending_projection_repo_ids};
#[allow(unused_imports)]
pub(crate) use types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportBaseline, RemoteImportBlocker,
    RemoteImportCandidateRevision, RemoteImportChangeKind, RemoteImportDigest,
    RemoteImportPrepareRequest, RemoteImportProjectionOutcome, RemoteImportRefreshRequest,
    RemoteImportSessionId, RemoteImportSessionRecord, RemoteImportSourceSnapshot,
    RemoteImportState,
};

#[cfg(test)]
mod tests;
