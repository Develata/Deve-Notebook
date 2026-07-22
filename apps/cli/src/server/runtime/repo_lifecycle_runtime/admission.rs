//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#remote-import-repo-lifecycle
//!
//! Exact catalog and Remote Import admission projections used by lifecycle.

use super::{RepoLifecycleCoordinator, RepoLifecycleError};
use deve_core::models::RepoId;
#[cfg(test)]
use deve_core::remote_import::{RemoteImportRepoRemovalAdmission, RemoteImportRepoRemovalSnapshot};
use std::path::Path;

impl RepoLifecycleCoordinator {
    pub(crate) fn revalidate_create_projection_base(
        &self,
        source_repo_id: Option<RepoId>,
        prepared_base: &Path,
    ) -> Result<(), RepoLifecycleError> {
        let current = if let Some(repo_id) = source_repo_id {
            let execution_name =
                self.repo
                    .find_local_repo_name_by_id(repo_id)?
                    .ok_or_else(|| RepoLifecycleError::NotCommitted {
                        operation: "create projection-base admission",
                        detail: "projection-base source repo left the local catalog".to_string(),
                    })?;
            self.repo
                .projection_locator_for_local_repo(&execution_name)?
                .projection_base_abs
        } else {
            self.configured_projection_base.clone().ok_or_else(|| {
                RepoLifecycleError::NotCommitted {
                    operation: "create projection-base admission",
                    detail: "repo creation projection base is no longer configured".to_string(),
                }
            })?
        };
        if current != prepared_base {
            return Err(RepoLifecycleError::NotCommitted {
                operation: "create projection-base admission",
                detail: "projection-base binding changed while the lifecycle job was queued"
                    .to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
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

#[cfg(test)]
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
