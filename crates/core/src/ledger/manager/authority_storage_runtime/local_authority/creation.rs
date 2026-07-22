//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Membership-bound first creation of a new local authority.

use super::{
    LocalAuthorityError, LocalAuthorityRuntime, PreparedRepoAuthority, PreparedRepoAuthorityOrigin,
    RepoAuthoritySlot,
};
use crate::models::RepoId;
use redb::Database;
use std::sync::Arc;
use uuid::Uuid;

impl LocalAuthorityRuntime {
    pub(crate) fn create_repo_initialized<F>(
        &self,
        repo_id: RepoId,
        initialize: F,
    ) -> Result<PreparedRepoAuthority, LocalAuthorityError>
    where
        F: FnOnce(&Database) -> Result<(), LocalAuthorityError>,
    {
        let reservation_id = Uuid::new_v4();
        {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            if slots.contains_key(&repo_id) {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            slots.insert(repo_id, RepoAuthoritySlot::Opening { reservation_id });
        }
        let mut opening =
            super::reservation::OpeningReservation::new(&self.inner, repo_id, reservation_id);

        let opened = super::resource::open_resources(&self.inner.ledger_dir, repo_id, true);
        let initialized = match opened {
            Ok(resources) => {
                opening.require_repair_on_drop();
                match initialize(resources.db()) {
                    Ok(()) => Ok(Arc::new(resources)),
                    Err(error) => {
                        drop(resources);
                        Err((error, true))
                    }
                }
            }
            Err(error) => {
                let db_path = super::resource::database_path(&self.inner.ledger_dir, repo_id);
                let lock_path =
                    super::resource::authority_lock_path(&self.inner.ledger_dir, repo_id);
                let residual =
                    db_path.try_exists().unwrap_or(true) || lock_path.try_exists().unwrap_or(true);
                Err((error, residual))
            }
        };
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if !matches!(
            slots.get(&repo_id),
            Some(RepoAuthoritySlot::Opening {
                reservation_id: current,
            }) if *current == reservation_id
        ) {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority create reservation changed for RepoId {repo_id}"
            )));
        }
        match initialized {
            Ok(resources) => {
                slots.insert(
                    repo_id,
                    RepoAuthoritySlot::Preparing {
                        reservation_id,
                        generation: 1,
                        resources: resources.clone(),
                    },
                );
                Ok(PreparedRepoAuthority {
                    inner: self.inner.clone(),
                    resources,
                    reservation_id,
                    repo_id,
                    generation: 1,
                    origin: PreparedRepoAuthorityOrigin::New,
                    settled: false,
                })
            }
            Err((error, repair_required)) => {
                if repair_required {
                    slots.insert(repo_id, RepoAuthoritySlot::RepairRequired { generation: 1 });
                } else {
                    slots.remove(&repo_id);
                }
                Err(error)
            }
        }
        .inspect(|_| opening.settle_after_transition())
        .inspect_err(|_| opening.settle_after_transition())
    }
}
