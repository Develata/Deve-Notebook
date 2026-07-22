//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-removal-owner-plan
//!
//! Removal-specific Remote Import admission and owner cleanup coordination.

use super::{ProviderQuiesceToken, RemoteImportCoordinator, RemoteImportHostError};
use deve_core::models::RepoId;
use deve_core::remote_import::{
    RemoteImportRepoRemovalAdmission, RemoteImportRepoRemovalCheckpoint,
    RemoteImportRepoRemovalPlan, RemoteImportRepoRemovalRevalidation,
    RemoteImportRepoRemovalSnapshot, RemoteImportService,
};

impl RemoteImportCoordinator {
    pub(crate) fn repo_removal_admission(
        &self,
        repo_id: RepoId,
    ) -> Result<RemoteImportRepoRemovalAdmission, RemoteImportHostError> {
        self.reject_active_apply(repo_id)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.repo_removal_admission()?)
    }

    pub(crate) fn revalidate_repo_removal(
        &self,
        repo_id: RepoId,
        expected: &RemoteImportRepoRemovalSnapshot,
    ) -> Result<RemoteImportRepoRemovalRevalidation, RemoteImportHostError> {
        self.reject_active_apply(repo_id)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.revalidate_repo_removal(expected)?)
    }

    /// Closes provider admission for one repo and waits until an in-flight
    /// immutable capture has either sealed or aborted. The returned token is
    /// exact process-local evidence for the remove commit phase.
    pub(crate) fn quiesce_provider_for_remove(
        &self,
        repo_id: RepoId,
    ) -> Result<ProviderQuiesceToken, RemoteImportHostError> {
        self.providers.quiesce(repo_id).map_err(Into::into)
    }

    pub(crate) fn resume_provider_after_failed_remove(
        &self,
        token: &ProviderQuiesceToken,
    ) -> Result<(), RemoteImportHostError> {
        self.providers.resume(token).map_err(Into::into)
    }

    pub(crate) fn finish_provider_after_remove(
        &self,
        token: ProviderQuiesceToken,
    ) -> Result<(), RemoteImportHostError> {
        self.providers.finish(token).map_err(Into::into)
    }

    pub(crate) fn seal_repo_removal(
        &self,
        repo_id: RepoId,
        expected: &RemoteImportRepoRemovalSnapshot,
    ) -> Result<RemoteImportRepoRemovalPlan, RemoteImportHostError> {
        self.reject_active_apply(repo_id)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.seal_repo_removal(expected)?)
    }

    pub(crate) fn advance_repo_removal(
        &self,
        plan: &RemoteImportRepoRemovalPlan,
        checkpoint: &RemoteImportRepoRemovalCheckpoint,
    ) -> Result<RemoteImportRepoRemovalCheckpoint, RemoteImportHostError> {
        Ok(RemoteImportService::advance_repo_removal(plan, checkpoint)?)
    }

    pub(crate) fn verify_repo_removal_complete(
        &self,
        plan: &RemoteImportRepoRemovalPlan,
        checkpoint: &RemoteImportRepoRemovalCheckpoint,
    ) -> Result<(), RemoteImportHostError> {
        Ok(RemoteImportService::verify_repo_removal_complete(
            plan, checkpoint,
        )?)
    }

    pub(crate) fn revalidate_sealed_repo_removal(
        &self,
        repo_id: RepoId,
        expected: &RemoteImportRepoRemovalSnapshot,
        plan: &RemoteImportRepoRemovalPlan,
    ) -> Result<bool, RemoteImportHostError> {
        self.reject_active_apply(repo_id)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?
            .revalidate_sealed_repo_removal(expected, plan)?)
    }

    pub(crate) fn invalidate_repo_removal(
        &self,
        plan: &RemoteImportRepoRemovalPlan,
    ) -> Result<(), RemoteImportHostError> {
        Ok(RemoteImportService::invalidate_repo_removal(plan)?)
    }

    pub(crate) fn cleanup_removed_provider_runtime(
        &self,
        repo_id: RepoId,
    ) -> Result<bool, RemoteImportHostError> {
        self.providers
            .cleanup_removed_repo(repo_id)
            .map_err(Into::into)
    }

    pub(crate) fn provider_admission_is_open(
        &self,
        repo_id: RepoId,
    ) -> Result<bool, RemoteImportHostError> {
        self.providers
            .admission_is_open(repo_id)
            .map_err(Into::into)
    }

    pub(crate) fn removed_provider_runtime_is_absent(
        &self,
        repo_id: RepoId,
    ) -> Result<bool, RemoteImportHostError> {
        self.providers
            .removed_repo_slot_is_absent(repo_id)
            .map_err(Into::into)
    }

    fn reject_active_apply(&self, repo_id: RepoId) -> Result<(), RemoteImportHostError> {
        if self
            .applying
            .lock()
            .map_err(|_| RemoteImportHostError::Coordination)?
            .contains(&repo_id)
        {
            Err(RemoteImportHostError::ApplyBusy)
        } else {
            Ok(())
        }
    }
}
