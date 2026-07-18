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
mod facade;
mod manifest;
mod repair;
mod runtime;
mod store;
mod types;

pub use error::{RemoteImportError, RemoteImportResult};
pub use facade::{
    REMOTE_IMPORT_DEFAULT_PAGE_SIZE, REMOTE_IMPORT_MAX_PAGE_SIZE, RemoteImportApplyView,
    RemoteImportBinding, RemoteImportCandidatePage, RemoteImportCandidateView,
    RemoteImportCaptureSink, RemoteImportDiffView, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportRepairPlan, RemoteImportService, RemoteImportSessionView,
};
#[allow(unused_imports)]
pub(crate) use repair::{RemoteImportRepairFinding, RemoteImportRepairReport};
#[allow(unused_imports)]
pub(crate) use runtime::{RemoteImportCapture, RemoteImportRuntime, pending_projection_repo_ids};
#[allow(unused_imports)]
pub(crate) use types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportBaseline, RemoteImportDigest,
    RemoteImportPrepareRequest, RemoteImportRefreshRequest, RemoteImportSessionRecord,
    RemoteImportSourceSnapshot,
};
pub use types::{
    RemoteImportBlocker, RemoteImportCandidateRevision, RemoteImportChangeKind,
    RemoteImportProjectionOutcome, RemoteImportSessionId, RemoteImportState,
};

#[cfg(test)]
mod tests;
