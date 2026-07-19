//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/authority#repo-mutation-publication-gate

use super::error::state_name;
use super::model::{
    PreparedRepoCreation, PreparedRepoRemoval, RepoCatalogCreationCommit,
    RepoCatalogMembershipRecord, RepoCatalogMembershipState, RepoCatalogRemovalCommit,
    RevalidatedRepoCreation, RevalidatedRepoRemoval,
};
use super::prepared_identity;
use super::store::{RepoCatalogPublishFailure, RepoCatalogPublishPhase, RepoCatalogStore};
use super::{CatalogMembershipError, RepoCatalogCutPermit, RepoCatalogError, RepoCatalogRuntime};
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use uuid::Uuid;

impl RepoManager {
    pub fn prepare_repo_creation_membership(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        self.repo_catalog_runtime()
            .prepare_creation(repo_id, lifecycle_request_id)
    }

    pub fn revalidate_repo_creation_membership(
        &self,
        prepared: &PreparedRepoCreation,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        self.repo_catalog_runtime().revalidate_creation(prepared)
    }

    /// Commits only the bounded repo-catalog sub-cut. B1 exposes no public
    /// constructor for `RepoCatalogCutPermit`; C1' transfers its sole owner
    /// into the ordered publication gate.
    pub fn commit_repo_creation_membership(
        &self,
        prepared: &PreparedRepoCreation,
        revalidated: &RevalidatedRepoCreation,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogCreationCommit, RepoCatalogError> {
        self.repo_catalog_runtime()
            .commit_creation(prepared, revalidated, permit)
    }

    pub fn prepare_repo_removal_membership(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoRemoval, RepoCatalogError> {
        self.repo_catalog_runtime()
            .prepare_removal(repo_id, lifecycle_request_id)
    }

    pub fn revalidate_repo_removal_membership(
        &self,
        prepared: &PreparedRepoRemoval,
    ) -> Result<RevalidatedRepoRemoval, RepoCatalogError> {
        self.repo_catalog_runtime().revalidate_removal(prepared)
    }

    /// Commits only the bounded repo-catalog sub-cut. The complete immutable
    /// removal plan, including initiator and fallback outcome, belongs to the
    /// host lifecycle coordinator in C1'.
    pub fn commit_repo_removal_membership(
        &self,
        prepared: &PreparedRepoRemoval,
        revalidated: &RevalidatedRepoRemoval,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRemovalCommit, RepoCatalogError> {
        self.repo_catalog_runtime()
            .commit_removal(prepared, revalidated, permit)
    }

    pub fn repo_catalog_membership_record(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoCatalogMembershipRecord>, RepoCatalogError> {
        self.repo_catalog_runtime().record(repo_id)
    }

    /// Seeds process readiness exclusively from durable catalog records.
    pub fn seed_catalog_membership_from_records(&self) -> Result<(), RepoCatalogError> {
        self.repo_catalog_runtime().seed_process_membership()
    }

    pub fn normal_repo_catalog_ids(&self) -> Result<Vec<RepoId>, RepoCatalogError> {
        self.repo_catalog_runtime().normal_repo_ids()
    }
}

impl RepoCatalogRuntime<'_> {
    fn prepare_creation(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        validate_request_id(lifecycle_request_id)?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        if store.load(repo_id)?.is_some() {
            return Err(RepoCatalogError::AlreadyExists(repo_id));
        }
        match self.manager.catalog_membership.issue(repo_id) {
            Err(CatalogMembershipError::NotMember(_)) => {}
            Err(error) => return Err(error.into()),
            Ok(_) => {
                return Err(RepoCatalogError::CutOutcomeUnknown {
                    repo_id,
                    detail: "process membership exists while durable record is absent".to_string(),
                });
            }
        }
        Ok(PreparedRepoCreation {
            repo_id,
            lifecycle_request_id,
            prepared_identity: prepared_identity::snapshot(self.manager, repo_id)?,
        })
    }

    fn revalidate_creation(
        &self,
        prepared: &PreparedRepoCreation,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        let observed = prepared_identity::snapshot(self.manager, prepared.repo_id)?;
        if observed != prepared.prepared_identity {
            return Err(RepoCatalogError::PreparedIdentityChanged(prepared.repo_id));
        }
        Ok(RevalidatedRepoCreation {
            repo_id: prepared.repo_id,
            lifecycle_request_id: prepared.lifecycle_request_id,
            prepared_identity: observed,
            store: RepoCatalogStore::open(&self.manager.ledger_dir)?,
        })
    }

    fn commit_creation(
        &self,
        prepared: &PreparedRepoCreation,
        revalidated: &RevalidatedRepoCreation,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogCreationCommit, RepoCatalogError> {
        validate_creation_revalidation(prepared, revalidated)?;
        self.manager
            .catalog_membership
            .validate_cut_permit(permit, prepared.repo_id)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = revalidated.store.lock()?;
        let desired = RepoCatalogMembershipRecord::normal(
            prepared.repo_id,
            prepared.prepared_identity,
            prepared.lifecycle_request_id,
        );
        match revalidated.store.load(prepared.repo_id)? {
            None => revalidated
                .store
                .publish(&desired)
                .map_err(|failure| publish_error(prepared.repo_id, failure))?,
            Some(current) if current == desired => revalidated.store.seal_visible_record()?,
            Some(_) => return Err(RepoCatalogError::AlreadyExists(prepared.repo_id)),
        }
        let membership = match self.manager.catalog_membership.issue(prepared.repo_id) {
            Ok(existing) => existing,
            Err(CatalogMembershipError::NotMember(_)) => self
                .manager
                .catalog_membership
                .admit_created(prepared.repo_id)
                .map_err(|error| RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id: prepared.repo_id,
                    detail: error.to_string(),
                })?,
            Err(error) => {
                return Err(RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id: prepared.repo_id,
                    detail: error.to_string(),
                });
            }
        };
        Ok(RepoCatalogCreationCommit::new(desired, membership))
    }

    fn prepare_removal(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoRemoval, RepoCatalogError> {
        validate_request_id(lifecycle_request_id)?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let normal_record = store
            .load(repo_id)?
            .ok_or(RepoCatalogError::NotFound(repo_id))?;
        require_state(&normal_record, RepoCatalogMembershipState::Normal)?;
        let prepared_identity = prepared_identity::snapshot(self.manager, repo_id)?;
        let membership = self.manager.catalog_membership.issue(repo_id)?;
        Ok(PreparedRepoRemoval {
            normal_record,
            membership,
            prepared_identity,
            lifecycle_request_id,
        })
    }

    fn revalidate_removal(
        &self,
        prepared: &PreparedRepoRemoval,
    ) -> Result<RevalidatedRepoRemoval, RepoCatalogError> {
        let repo_id = prepared.normal_record.repo_id;
        // A relocation after prepare changes this current removal snapshot, but
        // does not compare against the historical creation digest in record v1.
        let observed = prepared_identity::snapshot(self.manager, repo_id)?;
        if observed != prepared.prepared_identity {
            return Err(RepoCatalogError::PreparedIdentityChanged(repo_id));
        }
        Ok(RevalidatedRepoRemoval {
            repo_id,
            lifecycle_request_id: prepared.lifecycle_request_id,
            prepared_identity: observed,
            store: RepoCatalogStore::open(&self.manager.ledger_dir)?,
        })
    }

    fn commit_removal(
        &self,
        prepared: &PreparedRepoRemoval,
        revalidated: &RevalidatedRepoRemoval,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRemovalCommit, RepoCatalogError> {
        let repo_id = prepared.normal_record.repo_id;
        validate_removal_revalidation(prepared, revalidated)?;
        self.manager
            .catalog_membership
            .validate_cut_permit(permit, repo_id)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = revalidated.store.lock()?;
        let current = revalidated
            .store
            .load(repo_id)?
            .ok_or(RepoCatalogError::NotFound(repo_id))?;
        let desired = RepoCatalogMembershipRecord::removed(
            &prepared.normal_record,
            prepared.lifecycle_request_id,
        )?;
        if current == desired {
            revalidated.store.seal_visible_record()?;
            if !self
                .manager
                .catalog_membership
                .converge_removed(&prepared.membership)?
            {
                return Err(RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id,
                    detail: "durable record is removed but process generation did not converge"
                        .to_string(),
                });
            }
            return Ok(RepoCatalogRemovalCommit::new(
                desired,
                prepared.membership.clone(),
            ));
        }
        if current != prepared.normal_record {
            return Err(RepoCatalogError::CutOutcomeUnknown {
                repo_id,
                detail: "normal membership record changed after prepare".to_string(),
            });
        }
        let revocation = self
            .manager
            .catalog_membership
            .begin_removal(&prepared.membership)?;
        match revalidated.store.publish(&desired) {
            Ok(()) => self
                .manager
                .catalog_membership
                .finalize_removed(&revocation)
                .map_err(|error| RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id,
                    detail: error.to_string(),
                })?,
            Err(failure) if failure.phase == RepoCatalogPublishPhase::BeforeReplace => {
                let abort = self
                    .manager
                    .catalog_membership
                    .abort_removal(&revocation)
                    .err()
                    .map(|error| error.to_string());
                return Err(publish_error_with_abort(repo_id, failure, abort));
            }
            Err(failure) => {
                // The replacement is visible but crash durability is unknown.
                // Leave admission fail-closed in Revoking until retry/restart
                // rereads truth and explicitly seals the directory.
                return Err(publish_error(repo_id, failure));
            }
        }
        Ok(RepoCatalogRemovalCommit::new(
            desired,
            prepared.membership.clone(),
        ))
    }

    fn record(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoCatalogMembershipRecord>, RepoCatalogError> {
        RepoCatalogStore::open(&self.manager.ledger_dir)?.load(repo_id)
    }

    fn seed_process_membership(&self) -> Result<(), RepoCatalogError> {
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = store.lock()?;
        let repo_ids = normal_ids(store.list()?);
        self.manager.catalog_membership.seed(repo_ids)?;
        Ok(())
    }

    fn normal_repo_ids(&self) -> Result<Vec<RepoId>, RepoCatalogError> {
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = store.lock()?;
        Ok(normal_ids(store.list()?))
    }
}

fn normal_ids(records: Vec<RepoCatalogMembershipRecord>) -> Vec<RepoId> {
    records
        .into_iter()
        .filter_map(|record| {
            (record.state == RepoCatalogMembershipState::Normal).then_some(record.repo_id)
        })
        .collect()
}

fn validate_creation_revalidation(
    prepared: &PreparedRepoCreation,
    revalidated: &RevalidatedRepoCreation,
) -> Result<(), RepoCatalogError> {
    if revalidated.repo_id == prepared.repo_id
        && revalidated.lifecycle_request_id == prepared.lifecycle_request_id
        && revalidated.prepared_identity == prepared.prepared_identity
    {
        Ok(())
    } else {
        Err(RepoCatalogError::PreparedIdentityChanged(prepared.repo_id))
    }
}

fn validate_removal_revalidation(
    prepared: &PreparedRepoRemoval,
    revalidated: &RevalidatedRepoRemoval,
) -> Result<(), RepoCatalogError> {
    let repo_id = prepared.normal_record.repo_id;
    if revalidated.repo_id == repo_id
        && revalidated.lifecycle_request_id == prepared.lifecycle_request_id
        && revalidated.prepared_identity == prepared.prepared_identity
    {
        Ok(())
    } else {
        Err(RepoCatalogError::PreparedIdentityChanged(repo_id))
    }
}

fn publish_error(repo_id: RepoId, failure: RepoCatalogPublishFailure) -> RepoCatalogError {
    publish_error_with_abort(repo_id, failure, None)
}

fn publish_error_with_abort(
    repo_id: RepoId,
    failure: RepoCatalogPublishFailure,
    abort: Option<String>,
) -> RepoCatalogError {
    let phase = match failure.phase {
        RepoCatalogPublishPhase::BeforeReplace => "before_replace",
        RepoCatalogPublishPhase::AfterReplaceSync => "after_replace_sync",
    };
    let mut cleanup = failure.cleanup.map(|error| error.to_string());
    if let Some(abort) = abort {
        cleanup = Some(match cleanup {
            Some(cleanup) => format!("temp_cleanup={cleanup}; membership_abort={abort}"),
            None => format!("membership_abort={abort}"),
        });
    }
    match failure.phase {
        RepoCatalogPublishPhase::BeforeReplace => RepoCatalogError::PublishFailed {
            repo_id,
            phase,
            primary: failure.primary.to_string(),
            cleanup,
        },
        RepoCatalogPublishPhase::AfterReplaceSync => RepoCatalogError::CutOutcomeUnknown {
            repo_id,
            detail: match cleanup {
                Some(cleanup) => format!(
                    "phase={phase}; primary={}; cleanup={cleanup}",
                    failure.primary
                ),
                None => format!("phase={phase}; primary={}", failure.primary),
            },
        },
    }
}

fn validate_request_id(request_id: Uuid) -> Result<(), RepoCatalogError> {
    if request_id.is_nil() {
        Err(RepoCatalogError::InvalidRecord(
            "lifecycle request_id must not be nil".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn require_state(
    record: &RepoCatalogMembershipRecord,
    expected: RepoCatalogMembershipState,
) -> Result<(), RepoCatalogError> {
    if record.state == expected {
        Ok(())
    } else {
        Err(RepoCatalogError::StateMismatch {
            repo_id: record.repo_id,
            expected: state_name(expected),
            actual: state_name(record.state),
        })
    }
}
