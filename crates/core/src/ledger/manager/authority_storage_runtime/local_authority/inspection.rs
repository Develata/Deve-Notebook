//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-catalog-repair-contract
//!
//! Bounded physical inspection for bootstrap and explicit repair. It never
//! publishes an ordinary Active slot and cannot return a database capability.

use super::{LocalAuthorityError, LocalAuthorityRuntime, RepoAuthoritySlot};
use crate::models::RepoId;
use redb::Database;
use uuid::Uuid;

impl LocalAuthorityRuntime {
    pub(crate) fn inspect_existing_stem<F, R>(&self, stem: &str, inspect: F) -> anyhow::Result<R>
    where
        F: FnOnce(&Database) -> anyhow::Result<R>,
    {
        let repo_id = parse_canonical_repo_id(stem)?;
        match self.lease(repo_id) {
            Ok(lease) => return inspect(lease.db()),
            Err(LocalAuthorityError::NotAdmitted(_) | LocalAuthorityError::RepairRequired(_)) => {}
            Err(error) => return Err(error.into()),
        }

        let reservation_id = Uuid::new_v4();
        let repair_generation = {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            let repair_generation = match slots.get(&repo_id) {
                None => None,
                Some(RepoAuthoritySlot::RepairRequired { generation }) => Some(*generation),
                Some(_) => return Err(LocalAuthorityError::Busy(repo_id).into()),
            };
            slots.insert(repo_id, RepoAuthoritySlot::Opening { reservation_id });
            repair_generation
        };
        let mut opening =
            super::reservation::OpeningReservation::new(&self.inner, repo_id, reservation_id);
        if let Some(generation) = repair_generation {
            opening.restore_repair_on_drop(generation);
        }

        let opened = super::resource::open_resources(&self.inner.ledger_dir, repo_id, false)
            .and_then(|resources| {
                super::resource::validate_existing(resources.db_path.as_path(), resources.db())?;
                Ok(resources)
            });
        let result = match opened {
            Ok(resources) => {
                let result = inspect(resources.db());
                drop(resources);
                result
            }
            Err(error) => Err(error.into()),
        };
        let cleanup = match repair_generation {
            Some(generation) => opening.restore_repair(generation),
            None => opening.remove(),
        };
        match (result, cleanup) {
            (Err(primary), Err(cleanup)) => {
                tracing::error!(%repo_id, %primary, %cleanup, "physical authority inspection cleanup also failed");
                Err(primary)
            }
            (Err(primary), Ok(())) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup.into()),
            (Ok(value), Ok(())) => Ok(value),
        }
    }
}

fn parse_canonical_repo_id(stem: &str) -> Result<RepoId, LocalAuthorityError> {
    let repo_id = Uuid::parse_str(stem).map_err(|_| {
        LocalAuthorityError::Invariant(format!(
            "physical authority selector is not a RepoId: {stem}"
        ))
    })?;
    if repo_id.to_string() != stem {
        return Err(LocalAuthorityError::Invariant(format!(
            "physical authority selector is not canonical: {stem}"
        )));
    }
    Ok(repo_id)
}
