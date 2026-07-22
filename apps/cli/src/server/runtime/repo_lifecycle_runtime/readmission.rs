//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Sole production composition path for same-process Retired RepoId
//! reincarnation. No wire or UI command exposes this capability.

use super::{
    CreateRepoOutcome, ReadmitRetiredRepoIntent, RepoLifecycleCoordinator, RepoLifecycleError,
};

impl RepoLifecycleCoordinator {
    #[allow(dead_code)] // Called by the Option A integration producer; no product trigger yet.
    pub(crate) async fn readmit_retired_repo(
        &self,
        intent: ReadmitRetiredRepoIntent,
    ) -> Result<CreateRepoOutcome, RepoLifecycleError> {
        let repo_id = intent.repo_id;
        let reservation = self
            .gate
            .execute_catalog_repo_unpublished(repo_id, || {
                if self.repo.repo_catalog_membership_record(repo_id)?.is_some() {
                    return Err(RepoLifecycleError::NotCommitted {
                        operation: "retired readmission",
                        detail: format!("repository already has durable catalog state: {repo_id}"),
                    });
                }
                Ok(self.watchers.reserve_new(repo_id)?)
            })
            .await??;

        let prepared = match self.repo.prepare_retired_local_repo_reincarnation(
            repo_id,
            &intent.projection_base,
            intent.repo_url,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    repo_id,
                    "retired readmission prepare",
                    error.to_string(),
                ));
            }
        };
        let creation = match self.repo.prepare_repo_creation_membership_with_authority(
            repo_id,
            intent.lifecycle_request_id,
            &prepared,
        ) {
            Ok(creation) => creation,
            Err(error) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    repo_id,
                    "retired readmission membership prepare",
                    error.to_string(),
                ));
            }
        };
        let revalidated = match self
            .repo
            .revalidate_repo_creation_membership_with_authority(&creation, &prepared)
        {
            Ok(revalidated) => revalidated,
            Err(error) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    repo_id,
                    "retired readmission membership revalidate",
                    error.to_string(),
                ));
            }
        };
        let cut = self
            .gate
            .execute_catalog_repo_cut(repo_id, |permit| {
                self.repo
                    .commit_repo_creation_membership(&creation, &revalidated, permit)
            })
            .await;
        let commit = match cut {
            Ok(Ok(commit)) => commit,
            result => {
                let detail = match result {
                    Ok(Err(error)) => error.to_string(),
                    Err(error) => error.to_string(),
                    Ok(Ok(_)) => unreachable!("success handled above"),
                };
                super::mount::mark_repair_required(
                    &self.watchers,
                    &reservation,
                    format!("retired readmission catalog outcome is unknown: {detail}"),
                );
                return Err(RepoLifecycleError::RepairRequired {
                    operation: "retired readmission catalog cut",
                    repo_id,
                    detail,
                });
            }
        };

        if let Err(error) = self
            .repo
            .activate_prepared_local_repo_authority(prepared, &creation, &commit)
        {
            let detail =
                format!("fresh catalog membership committed but owner activation failed: {error}");
            super::mount::mark_repair_required(&self.watchers, &reservation, detail.clone());
            return Err(RepoLifecycleError::RepairRequired {
                operation: "retired readmission activation",
                repo_id,
                detail,
            });
        }

        if let Err(error) =
            self.repo
                .host_repo_alias_runtime()
                .set_alias(repo_id, &intent.initial_alias, 0)
        {
            tracing::error!(%repo_id, %error, "retired repo activated but local alias settlement failed");
        }
        let mount = self.mount(reservation, repo_id.to_string()).await?;
        Ok(CreateRepoOutcome { mount })
    }
}
