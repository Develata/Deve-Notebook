//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#remote-import-repo-lifecycle
//!
//! Typed host outcomes for dynamic local-repo lifecycle operations.

use crate::remote_import_runtime::RemoteImportHostError;
#[cfg(test)]
use crate::server::repo_mutation::MountedRepoAdmission;
use crate::server::repo_mutation::RepoMutationGateError;
use crate::server::runtime::watcher_runtime::WatcherLifecycleError;
use deve_core::ledger::{CatalogMembershipError, HostRepoAliasError, RepoCatalogError};
#[cfg(test)]
use deve_core::ledger::{CatalogMembershipToken, LocalRepoSummary};
use deve_core::models::RepoId;
#[cfg(test)]
use deve_core::remote_import::RemoteImportRepoRemovalBlocker;
use std::path::PathBuf;

pub(crate) struct CreateRepoIntent {
    pub(crate) repo_id: RepoId,
    pub(crate) initial_alias: String,
    pub(crate) projection_base: PathBuf,
    pub(crate) lifecycle_request_id: uuid::Uuid,
}

pub(crate) struct CreateRepoOutcome {
    pub(crate) mount: RepoMountOutcome,
}

#[allow(dead_code)] // Option A's compiled producer is exercised by the integration harness.
pub(crate) struct ReadmitRetiredRepoIntent {
    pub(crate) repo_id: RepoId,
    pub(crate) initial_alias: String,
    pub(crate) projection_base: PathBuf,
    pub(crate) lifecycle_request_id: uuid::Uuid,
    pub(crate) repo_url: Option<String>,
}

#[cfg(test)]
pub(crate) struct RepoRemovalFallback {
    summary: LocalRepoSummary,
    membership: CatalogMembershipToken,
    mount: MountedRepoAdmission,
}

#[cfg(test)]
impl RepoRemovalFallback {
    pub(super) fn new(
        summary: LocalRepoSummary,
        membership: CatalogMembershipToken,
        mount: MountedRepoAdmission,
    ) -> Self {
        debug_assert_eq!(summary.repo_id, membership.repo_id());
        debug_assert_eq!(summary.repo_id, mount.repo_id());
        Self {
            summary,
            membership,
            mount,
        }
    }

    pub(crate) fn summary(&self) -> &LocalRepoSummary {
        &self.summary
    }

    pub(crate) fn revalidate_outside_cut(
        &self,
        repo: &deve_core::ledger::RepoManager,
    ) -> Result<(), RepoLifecycleError> {
        let current = repo
            .get_local_repo_info_by_id(self.summary.repo_id)?
            .ok_or_else(|| RepoLifecycleError::NotCommitted {
                operation: "remove publication",
                detail: "fallback repo left the local catalog".to_string(),
            })?;
        if current.name != self.summary.name {
            return Err(RepoLifecycleError::NotCommitted {
                operation: "remove publication",
                detail: "fallback repo name binding changed".to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn revalidate_cut(
        &self,
        membership: &deve_core::ledger::CatalogMembershipRuntime,
    ) -> Result<(), RepoLifecycleError> {
        membership.revalidate(&self.membership)?;
        self.mount.revalidate()?;
        Ok(())
    }

    pub(crate) fn revalidate(
        &self,
        repo: &deve_core::ledger::RepoManager,
        membership: &deve_core::ledger::CatalogMembershipRuntime,
    ) -> Result<(), RepoLifecycleError> {
        self.revalidate_outside_cut(repo)?;
        self.revalidate_cut(membership)
    }
}

#[cfg(test)]
pub(crate) struct RemoveRepoOutcome {
    pub(crate) fallback: Option<RepoRemovalFallback>,
    #[allow(dead_code)] // Legacy safety regression fixture; production R3 has no soft-remove path.
    pub(crate) repair_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoMountOutcome {
    Mounted,
    Failed,
}

impl RepoMountOutcome {
    pub(crate) const fn is_mounted(self) -> bool {
        matches!(self, Self::Mounted)
    }
}

#[derive(Debug)]
pub(crate) enum RepoLifecycleError {
    Gate(RepoMutationGateError),
    Watcher(WatcherLifecycleError),
    Membership(CatalogMembershipError),
    Catalog(RepoCatalogError),
    Alias(HostRepoAliasError),
    RemoteImport(RemoteImportHostError),
    #[cfg(test)]
    RemoteImportBlocked(Vec<RemoteImportRepoRemovalBlocker>),
    NotCommitted {
        operation: &'static str,
        detail: String,
    },
    RepairRequired {
        operation: &'static str,
        repo_id: RepoId,
        detail: String,
    },
    Coordination(&'static str),
}

impl From<anyhow::Error> for RepoLifecycleError {
    fn from(error: anyhow::Error) -> Self {
        Self::NotCommitted {
            operation: "lifecycle",
            detail: error.to_string(),
        }
    }
}

impl std::fmt::Display for RepoLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gate(error) => error.fmt(formatter),
            Self::Watcher(error) => error.fmt(formatter),
            Self::Membership(error) => error.fmt(formatter),
            Self::Catalog(error) => error.fmt(formatter),
            Self::Alias(error) => error.fmt(formatter),
            Self::RemoteImport(error) => error.fmt(formatter),
            #[cfg(test)]
            Self::RemoteImportBlocked(blockers) => write!(
                formatter,
                "repository removal is blocked by {} Remote Import condition(s)",
                blockers.len()
            ),
            Self::NotCommitted { operation, detail } => {
                write!(
                    formatter,
                    "repository {operation} was not committed: {detail}"
                )
            }
            Self::RepairRequired {
                operation,
                repo_id,
                detail,
            } => write!(
                formatter,
                "repository {operation} for {repo_id} requires repair: {detail}"
            ),
            Self::Coordination(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for RepoLifecycleError {}

impl From<RepoMutationGateError> for RepoLifecycleError {
    fn from(error: RepoMutationGateError) -> Self {
        Self::Gate(error)
    }
}

impl From<WatcherLifecycleError> for RepoLifecycleError {
    fn from(error: WatcherLifecycleError) -> Self {
        Self::Watcher(error)
    }
}

impl From<CatalogMembershipError> for RepoLifecycleError {
    fn from(error: CatalogMembershipError) -> Self {
        Self::Membership(error)
    }
}

impl From<RepoCatalogError> for RepoLifecycleError {
    fn from(error: RepoCatalogError) -> Self {
        Self::Catalog(error)
    }
}

impl From<HostRepoAliasError> for RepoLifecycleError {
    fn from(error: HostRepoAliasError) -> Self {
        Self::Alias(error)
    }
}

impl From<RemoteImportHostError> for RepoLifecycleError {
    fn from(error: RemoteImportHostError) -> Self {
        Self::RemoteImport(error)
    }
}
