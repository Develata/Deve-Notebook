//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 03_storage/authority#remote-import-workflow-tables
//!
//! Backend-only immutable Remote Import session runtime.
//!
//! This module owns host artifacts and workflow state only. It deliberately has no provider,
//! protocol, workspace, External Changes, Source Control, or Ledger authority dependency.

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
pub(crate) use runtime::{RemoteImportCapture, RemoteImportRuntime};
#[allow(unused_imports)]
pub(crate) use types::{
    RemoteImportApplyReceipt, RemoteImportBaseline, RemoteImportBlocker,
    RemoteImportCandidateRevision, RemoteImportChangeKind, RemoteImportDigest,
    RemoteImportPrepareRequest, RemoteImportProjectionOutcome, RemoteImportRefreshRequest,
    RemoteImportSessionId, RemoteImportSessionRecord, RemoteImportSourceSnapshot,
    RemoteImportState,
};

#[cfg(test)]
mod tests;
