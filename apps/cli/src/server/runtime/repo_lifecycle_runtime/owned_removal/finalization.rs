//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Terminal-candidate verification after the authority owner lock is released.

use super::super::RepoLifecycleCoordinator;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobCompletion, RepoRemovalExecution,
};

impl RepoLifecycleCoordinator {
    #[cfg(test)]
    pub(crate) fn fail_next_authority_retirement_for_test(&self) {
        self.fail_next_authority_retirement
            .store(true, std::sync::atomic::Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn fail_next_terminal_completion_for_test(&self) {
        self.fail_next_terminal_completion
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub(super) async fn finalize_owned_removal_candidate(
        &self,
        removal: &mut RepoRemovalExecution,
        completion: RepoLifecycleJobCompletion,
    ) -> RepoLifecycleJobCompletion {
        let repo_id = removal.manifest.repo_id;
        match self.repo.repo_catalog_membership_record(repo_id) {
            Ok(None) => {}
            Ok(Some(_)) => {
                return RepoLifecycleJobCompletion::repair_required(
                    "TerminalCandidate catalog tombstone was not retired",
                );
            }
            Err(error) => {
                return RepoLifecycleJobCompletion::repair_required(format!(
                    "TerminalCandidate catalog truth is unreadable: {error}"
                ));
            }
        }
        if !self
            .repo
            .projection_locator_removal_is_absent(&removal.manifest.locator)
            .unwrap_or(false)
            || !self
                .repo
                .host_repo_alias_runtime()
                .removal_is_absent(&removal.manifest.alias)
                .unwrap_or(false)
        {
            return RepoLifecycleJobCompletion::repair_required(
                "TerminalCandidate owner rows no longer match retired truth",
            );
        }
        let remote_plan = match removal.state.remote_import_plan.as_ref() {
            Some(plan) => plan,
            None => {
                return RepoLifecycleJobCompletion::repair_required(
                    "TerminalCandidate lost its Remote Import plan",
                );
            }
        };
        let remote_checkpoint = match removal.state.remote_import_checkpoint.as_ref() {
            Some(checkpoint) => checkpoint,
            None => {
                return RepoLifecycleJobCompletion::repair_required(
                    "TerminalCandidate lost its Remote Import checkpoint",
                );
            }
        };
        if self
            .remote_import
            .verify_repo_removal_complete(remote_plan, remote_checkpoint)
            .is_err()
            || removal
                .state
                .notegit_checkpoint
                .as_ref()
                .is_none_or(|checkpoint| {
                    removal
                        .manifest
                        .notegit
                        .verify_complete(checkpoint)
                        .is_err()
                })
        {
            return RepoLifecycleJobCompletion::repair_required(
                "TerminalCandidate filesystem owner truth is not complete",
            );
        }
        let proof = match self
            .repo
            .acquire_local_authority_retirement_proof(&removal.manifest.authority)
        {
            Ok(proof) => proof,
            Err(error) => {
                return RepoLifecycleJobCompletion::repair_required(format!(
                    "cannot reacquire retired authority finalization lock: {error}"
                ));
            }
        };
        if proof.repo_id() != repo_id
            || proof.generation() != removal.manifest.authority.generation()
            || removal
                .state
                .authority_checkpoint
                .as_ref()
                .is_none_or(|checkpoint| {
                    !removal
                        .manifest
                        .authority
                        .verify_database_cleanup_complete(checkpoint)
                        .unwrap_or(false)
                })
        {
            return RepoLifecycleJobCompletion::repair_required(
                "TerminalCandidate authority proof does not bind its manifest",
            );
        }
        #[cfg(test)]
        if let Err(error) = self.install_terminal_completion_failure_for_test() {
            return RepoLifecycleJobCompletion::repair_required(error);
        }
        if let Err(error) = removal.progress.terminal_complete().await {
            return RepoLifecycleJobCompletion::repair_required(format!(
                "retired authority was verified but terminal receipt persistence failed: {error}"
            ));
        }
        drop(proof);
        completion
    }

    #[cfg(test)]
    pub(super) fn install_terminal_completion_failure_for_test(&self) -> Result<(), String> {
        if !self
            .fail_next_terminal_completion
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Ok(());
        }
        let marker = deve_core::utils::notegit::host_dir(self.repo.ledger_dir())
            .join("repo-lifecycle-jobs/removals")
            .join(
                crate::server::runtime::repo_lifecycle_job_runtime::REMOVAL_PRE_REPLACE_FAILURE_MARKER,
            );
        std::fs::write(&marker, b"inject").map_err(|error| {
            format!("could not install terminal-completion failure injection: {error}")
        })
    }
}
