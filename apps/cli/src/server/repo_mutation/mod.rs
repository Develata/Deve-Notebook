//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 07_network#projection-recovery-contract
//!
//! Process-scoped serialization and ordered publication for local repository
//! authority mutations.

mod binding;
mod execution;
mod gate;
mod plugin;
mod publication;

pub(crate) use binding::{
    prepare_writable_local_repo, revalidate_writable_local_repo, revalidate_writable_resolved_repo,
};
pub(crate) use execution::MutationExecution;
pub(crate) use gate::{
    MountedRepoAdmission, MountedRepoContinuation, RepoMutationGateError,
    RepoMutationPublicationGate,
};
#[cfg(not(test))]
pub(crate) use plugin::{CliManagedNoteMutationHost, CliManagedSourceControlMutationHost};
pub(crate) use publication::MutationPublication;

#[cfg(test)]
mod tests;
