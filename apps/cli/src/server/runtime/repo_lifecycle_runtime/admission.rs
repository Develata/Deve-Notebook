//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#remote-import-repo-lifecycle
//!
//! Exact catalog and Remote Import admission projections used by lifecycle.

use super::RepoLifecycleError;
use deve_core::remote_import::{RemoteImportRepoRemovalAdmission, RemoteImportRepoRemovalSnapshot};

pub(super) fn admitted_snapshot(
    admission: RemoteImportRepoRemovalAdmission,
) -> Result<RemoteImportRepoRemovalSnapshot, RepoLifecycleError> {
    match admission {
        RemoteImportRepoRemovalAdmission::Admitted(snapshot) => Ok(snapshot),
        RemoteImportRepoRemovalAdmission::Blocked(blocked) => Err(
            RepoLifecycleError::RemoteImportBlocked(blocked.blockers().to_vec()),
        ),
    }
}

pub(super) fn admission_error(admission: RemoteImportRepoRemovalAdmission) -> RepoLifecycleError {
    match admission {
        RemoteImportRepoRemovalAdmission::Blocked(blocked) => {
            RepoLifecycleError::RemoteImportBlocked(blocked.blockers().to_vec())
        }
        RemoteImportRepoRemovalAdmission::Admitted(_) => RepoLifecycleError::NotCommitted {
            operation: "remove",
            detail: "Remote Import removal admission changed before commit".to_string(),
        },
    }
}
