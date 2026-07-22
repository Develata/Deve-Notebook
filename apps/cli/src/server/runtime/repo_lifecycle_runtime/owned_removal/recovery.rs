//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Exact cut-outcome recovery and reverse-order pre-cut compensation.

use super::super::RepoLifecycleCoordinator;
use super::LiveRemovalReservation;
use crate::remote_import_runtime::ProviderQuiesceToken;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobCompletion, RepoRemovalExecution,
};
use crate::server::runtime::watcher_runtime::WatcherMountReservation;
use deve_core::ledger::{RepoAuthorityCleanupGuard, RepoAuthorityQuiesceGuard};
use deve_core::remote_import::RemoteImportRepoRemovalPlan;

impl RepoLifecycleCoordinator {
    #[cfg(test)]
    pub(crate) fn fail_after_cut_attempted_for_test(&self) {
        self.fail_after_cut_attempted
            .store(true, std::sync::atomic::Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn fail_after_catalog_cut_for_test(&self) {
        self.fail_after_catalog_cut
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub(super) async fn recover_attempted_cut(
        &self,
        removal: &mut RepoRemovalExecution,
    ) -> Result<
        (RepoAuthorityCleanupGuard, Option<LiveRemovalReservation>),
        RepoLifecycleJobCompletion,
    > {
        let repo_id = removal.manifest.repo_id;
        let current = self
            .repo
            .repo_catalog_membership_record(repo_id)
            .map_err(|error| {
                RepoLifecycleJobCompletion::repair_required(format!(
                    "cannot classify attempted removal cut: {error}"
                ))
            })?;
        if let Some(tombstone) = current.as_ref()
            && tombstone
                .confirms_removed_manifest(removal.execute_request_id, &removal.manifest_digest)
        {
            removal.state = removal
                .progress
                .cut_observed(tombstone.clone())
                .await
                .map_err(|error| RepoLifecycleJobCompletion::repair_required(error.to_string()))?;
            let guard = self
                .repo
                .resume_local_authority_cleanup(&removal.manifest.authority)
                .map_err(|error| {
                    RepoLifecycleJobCompletion::repair_required(format!(
                        "cannot resume observed authority cleanup: {error}"
                    ))
                })?;
            return Ok((guard, None));
        }
        if current.as_ref() == Some(&removal.manifest.catalog) {
            let runtime_restored = self.static_removal_owners_are_exact(removal)
                && self
                    .repo
                    .revalidate_local_authority_for_removal(&removal.manifest.authority)
                    .unwrap_or(false)
                && self
                    .watchers
                    .mounted_generation(repo_id)
                    .is_ok_and(|generation| generation == removal.manifest.watcher_generation)
                && self
                    .remote_import
                    .provider_admission_is_open(repo_id)
                    .unwrap_or(false);
            if !runtime_restored {
                return Err(RepoLifecycleJobCompletion::repair_required(
                    "attempted cut remained Normal but pre-cut runtime compensation is not exact",
                ));
            }
            let plan = removal.state.remote_import_plan.as_ref().ok_or_else(|| {
                RepoLifecycleJobCompletion::repair_required(
                    "attempted removal cut has no sealed Remote Import plan",
                )
            })?;
            self.remote_import
                .invalidate_repo_removal(plan)
                .map_err(|error| {
                    RepoLifecycleJobCompletion::repair_required(format!(
                        "attempted cut was not committed but owner-plan invalidation failed: {error}"
                    ))
                })?;
            removal.state = removal
                .progress
                .cut_not_committed()
                .await
                .map_err(|error| RepoLifecycleJobCompletion::repair_required(error.to_string()))?;
            return Err(RepoLifecycleJobCompletion::not_committed(
                "removal cut was attempted but exact catalog truth remained Normal",
            ));
        }
        Err(RepoLifecycleJobCompletion::repair_required(format!(
            "attempted removal cut has non-unique catalog truth: {current:?}"
        )))
    }

    pub(super) async fn rollback_owned_removal(
        &self,
        watcher: WatcherMountReservation,
        provider: Option<ProviderQuiesceToken>,
        authority: Option<RepoAuthorityQuiesceGuard>,
        remote_plan: Option<&RemoteImportRepoRemovalPlan>,
        primary: String,
    ) -> RepoLifecycleJobCompletion {
        let repair = |stage: &str, diagnostic: String| {
            RepoLifecycleJobCompletion::repair_required(primary.clone())
                .with_cleanup(format!("pre-cut {stage} failed: {diagnostic}"))
        };
        if let Some(authority) = authority
            && let Err(error) = authority.rollback()
        {
            return repair("authority rollback", error.to_string());
        }
        let execution_name = watcher.repo_id().to_string();
        let watcher = match self.start_mount_unfinalized(watcher, execution_name).await {
            Ok(watcher) => watcher,
            Err(error) => return repair("watcher restart", error.to_string()),
        };
        if let Some(plan) = remote_plan
            && let Err(error) = self.remote_import.invalidate_repo_removal(plan)
        {
            return repair("Remote Import plan invalidation", error.to_string());
        }
        if let Some(provider) = provider
            && let Err(error) = self.resume_provider(provider).await
        {
            return repair("provider resume", error.to_string());
        }
        match self.finalize_started_mount(watcher) {
            Ok(outcome) if outcome.is_mounted() => {
                RepoLifecycleJobCompletion::not_committed(primary)
            }
            Ok(_) => repair(
                "watcher remount",
                "watcher did not reach Mounted".to_string(),
            ),
            Err(error) => repair("watcher remount", error.to_string()),
        }
    }
}
