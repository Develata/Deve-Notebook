//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Read-only, owner-specific admission for explicit committed-debt repair.

use super::RepoLifecycleCoordinator;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RemovalCleanupStep, RemovalCutState, RemovalTerminalState, RepoLifecycleJobError,
    RepoRemovalExecution, RepoRemovalRepairInspection, RepoRemovalRepairItem,
    RepoRemovalRepairTarget, RepoRemovalRepairTruth,
};
use deve_core::ledger::RepoCatalogMembershipState;

impl RepoLifecycleCoordinator {
    pub(crate) fn inspect_owned_removal_repair(
        &self,
        removal: &RepoRemovalExecution,
    ) -> Result<RepoRemovalRepairInspection, RepoLifecycleJobError> {
        if matches!(removal.state.cut, RemovalCutState::NotAttempted)
            || !removal.state.has_committed_debt()
        {
            return Err(RepoLifecycleJobError::RemovalRepairNotRequired);
        }

        let mut remaining = Vec::new();
        let mut apply_allowed = true;
        let catalog = self
            .repo
            .repo_catalog_membership_record(removal.manifest.repo_id);
        let catalog_truth = match (catalog.as_ref(), &removal.state.cut) {
            (Err(_), _) => RepoRemovalRepairTruth::Unknown,
            (Ok(None), _) if removal.state.tombstone_retired => RepoRemovalRepairTruth::Exact,
            (Ok(None), _) => RepoRemovalRepairTruth::AlreadyAbsent,
            (Ok(Some(record)), RemovalCutState::Attempted)
                if !removal.state.tombstone_retired
                    && record.confirms_removed_manifest(
                        removal.execute_request_id,
                        &removal.manifest_digest,
                    ) =>
            {
                RepoRemovalRepairTruth::Exact
            }
            (Ok(Some(record)), RemovalCutState::Observed { tombstone })
                if !removal.state.tombstone_retired
                    && record == tombstone
                    && tombstone.state() == RepoCatalogMembershipState::Removed =>
            {
                RepoRemovalRepairTruth::Exact
            }
            _ => RepoRemovalRepairTruth::Changed,
        };
        if !removal.state.tombstone_retired || catalog_truth != RepoRemovalRepairTruth::Exact {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::CatalogTombstone,
                catalog_truth,
                &mut apply_allowed,
            );
        }

        let remote_complete = removal
            .state
            .completed(RemovalCleanupStep::RemoteImportArtifacts);
        let remote_truth = match removal
            .state
            .remote_import_plan
            .as_ref()
            .zip(removal.state.remote_import_checkpoint.as_ref())
        {
            Some((plan, checkpoint)) if remote_complete => {
                match self
                    .remote_import
                    .verify_repo_removal_complete(plan, checkpoint)
                {
                    Ok(()) => RepoRemovalRepairTruth::Exact,
                    Err(_) => RepoRemovalRepairTruth::Unknown,
                }
            }
            Some((plan, checkpoint)) => match self
                .remote_import
                .repo_removal_repair_retry_is_exact(plan, checkpoint)
            {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            },
            None => RepoRemovalRepairTruth::Unknown,
        };
        if !remote_complete || remote_truth != RepoRemovalRepairTruth::Exact {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::RemoteImportArtifacts,
                remote_truth,
                &mut apply_allowed,
            );
        }

        if !removal
            .state
            .completed(RemovalCleanupStep::ProcessRuntimeSlots)
        {
            remaining.push(RepoRemovalRepairItem {
                target: RepoRemovalRepairTarget::ProcessRuntimeSlots,
                truth: RepoRemovalRepairTruth::Exact,
            });
        } else {
            let process_truth = match (
                self.remote_import
                    .removed_provider_runtime_is_absent(removal.manifest.repo_id),
                self.watchers
                    .removed_repo_runtime_is_absent(removal.manifest.repo_id),
            ) {
                (Ok(true), Ok(true)) => RepoRemovalRepairTruth::Exact,
                (Ok(_), Ok(_)) => RepoRemovalRepairTruth::Changed,
                _ => RepoRemovalRepairTruth::Unknown,
            };
            if process_truth != RepoRemovalRepairTruth::Exact {
                push_truth(
                    &mut remaining,
                    RepoRemovalRepairTarget::ProcessRuntimeSlots,
                    process_truth,
                    &mut apply_allowed,
                );
            }
        }

        let notegit_checkpoint = removal
            .state
            .notegit_checkpoint
            .clone()
            .unwrap_or_else(|| removal.manifest.notegit.initial_checkpoint());
        let notegit_complete = removal.state.completed(RemovalCleanupStep::NotegitTree);
        let notegit_truth = if notegit_complete {
            match removal
                .manifest
                .notegit
                .verify_complete(&notegit_checkpoint)
            {
                Ok(()) => RepoRemovalRepairTruth::Exact,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            }
        } else {
            match removal
                .manifest
                .notegit
                .repair_retry_is_exact(&notegit_checkpoint)
            {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            }
        };
        if !notegit_complete || notegit_truth != RepoRemovalRepairTruth::Exact {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::NotegitTree,
                notegit_truth,
                &mut apply_allowed,
            );
        }

        let authority_checkpoint = removal
            .state
            .authority_checkpoint
            .clone()
            .unwrap_or_else(|| removal.manifest.authority.initial_database_checkpoint());
        let authority_complete = removal
            .state
            .completed(RemovalCleanupStep::LocalAuthorityDatabase);
        let authority_truth = if authority_complete {
            match removal
                .manifest
                .authority
                .verify_database_cleanup_complete(&authority_checkpoint)
            {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            }
        } else {
            match removal
                .manifest
                .authority
                .repair_retry_is_exact(&authority_checkpoint)
            {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            }
        };
        if !authority_complete || authority_truth != RepoRemovalRepairTruth::Exact {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::LocalAuthorityDatabase,
                authority_truth,
                &mut apply_allowed,
            );
        }

        let locator_truth = match self
            .repo
            .projection_locator_removal_retry_is_exact(&removal.manifest.locator)
        {
            Ok(true) => RepoRemovalRepairTruth::Exact,
            Ok(false) => match self
                .repo
                .projection_locator_removal_is_absent(&removal.manifest.locator)
            {
                Ok(true) => RepoRemovalRepairTruth::AlreadyAbsent,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            },
            Err(_) => RepoRemovalRepairTruth::Unknown,
        };
        if !removal
            .state
            .completed(RemovalCleanupStep::ProjectionLocator)
        {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::ProjectionLocator,
                locator_truth,
                &mut apply_allowed,
            );
        } else {
            let completed_truth = match self
                .repo
                .projection_locator_removal_is_absent(&removal.manifest.locator)
            {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            };
            if completed_truth != RepoRemovalRepairTruth::Exact {
                push_truth(
                    &mut remaining,
                    RepoRemovalRepairTarget::ProjectionLocator,
                    completed_truth,
                    &mut apply_allowed,
                );
            }
        }

        let alias_runtime = self.repo.host_repo_alias_runtime();
        let alias_truth = match alias_runtime.removal_retry_is_exact(&removal.manifest.alias) {
            Ok(true) => RepoRemovalRepairTruth::Exact,
            Ok(false) => match alias_runtime.removal_is_absent(&removal.manifest.alias) {
                Ok(true) => RepoRemovalRepairTruth::AlreadyAbsent,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            },
            Err(_) => RepoRemovalRepairTruth::Unknown,
        };
        if !removal.state.completed(RemovalCleanupStep::HostAlias) {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::HostAlias,
                alias_truth,
                &mut apply_allowed,
            );
        } else {
            let completed_truth = match alias_runtime.removal_is_absent(&removal.manifest.alias) {
                Ok(true) => RepoRemovalRepairTruth::Exact,
                Ok(false) => RepoRemovalRepairTruth::Changed,
                Err(_) => RepoRemovalRepairTruth::Unknown,
            };
            if completed_truth != RepoRemovalRepairTruth::Exact {
                push_truth(
                    &mut remaining,
                    RepoRemovalRepairTarget::HostAlias,
                    completed_truth,
                    &mut apply_allowed,
                );
            }
        }

        if !matches!(removal.state.terminal, RemovalTerminalState::Complete) {
            push_truth(
                &mut remaining,
                RepoRemovalRepairTarget::AuthorityRetirement,
                authority_truth,
                &mut apply_allowed,
            );
            remaining.push(RepoRemovalRepairItem {
                target: RepoRemovalRepairTarget::TerminalReceipt,
                truth: RepoRemovalRepairTruth::Exact,
            });
        }

        Ok(RepoRemovalRepairInspection {
            request_id: removal.execute_request_id,
            repo_id: removal.manifest.repo_id,
            remaining,
            apply_allowed,
        })
    }
}

fn push_truth(
    remaining: &mut Vec<RepoRemovalRepairItem>,
    target: RepoRemovalRepairTarget,
    truth: RepoRemovalRepairTruth,
    apply_allowed: &mut bool,
) {
    if truth != RepoRemovalRepairTruth::Exact {
        *apply_allowed = false;
    }
    remaining.push(RepoRemovalRepairItem { target, truth });
}
