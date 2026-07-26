//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Cold-host recovery and terminal retirement proof for committed cleanup.

use super::completion::slot_generation;
use crate::ledger::manager::authority_storage_runtime::local_authority::{
    LocalAuthorityError, LocalAuthorityRuntime, RepoAuthorityCleanupGuard,
    RepoAuthorityRemovalSnapshot, RepoAuthorityRetirementProof, RepoAuthoritySlot,
};

impl LocalAuthorityRuntime {
    /// Reacquires the persistent owner lock for an exact interrupted
    /// post-cut cleanup without opening the removed Redb authority.
    pub(crate) fn resume_committed_cleanup(
        &self,
        snapshot: &RepoAuthorityRemovalSnapshot,
    ) -> Result<RepoAuthorityCleanupGuard, LocalAuthorityError> {
        let repo_id = snapshot.repo_id();
        {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            if let Some(RepoAuthoritySlot::CommittedCleanup {
                generation,
                db_path,
                cleanup_capability_issued,
                ..
            }) = slots.get_mut(&repo_id)
            {
                if *generation != snapshot.generation()
                    || *db_path != snapshot.database().path()
                    || *cleanup_capability_issued
                {
                    return Err(LocalAuthorityError::Busy(repo_id));
                }
                *cleanup_capability_issued = true;
                return Ok(RepoAuthorityCleanupGuard {
                    inner: self.inner.clone(),
                    db_path: db_path.clone(),
                    repo_id,
                    generation: *generation,
                    settled: false,
                });
            }
            if slots.contains_key(&repo_id) {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
        }
        let (authority_lock, db_path) =
            super::super::resource::open_committed_cleanup_lock(&self.inner.ledger_dir, snapshot)?;
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if slots.contains_key(&repo_id) {
            return Err(LocalAuthorityError::Busy(repo_id));
        }
        slots.insert(
            repo_id,
            RepoAuthoritySlot::CommittedCleanup {
                generation: snapshot.generation(),
                authority_lock: std::sync::Arc::new(authority_lock),
                expected_lock_identity: snapshot.authority_lock().clone(),
                removed_database_identity: snapshot.database().clone(),
                db_path: db_path.clone(),
                cleanup_capability_issued: true,
            },
        );
        Ok(RepoAuthorityCleanupGuard {
            inner: self.inner.clone(),
            db_path,
            repo_id,
            generation: snapshot.generation(),
            settled: false,
        })
    }

    pub(crate) fn acquire_retired_finalization(
        &self,
        snapshot: &RepoAuthorityRemovalSnapshot,
    ) -> Result<RepoAuthorityRetirementProof, LocalAuthorityError> {
        let repo_id = snapshot.repo_id();
        self.validate_retired_slot(snapshot)?;
        let (authority_lock, db_path) =
            super::super::resource::open_committed_cleanup_lock(&self.inner.ledger_dir, snapshot)?;
        if db_path != snapshot.database().path()
            || !snapshot.database_quarantine.is_fully_absent()?
        {
            return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
        }
        self.validate_retired_slot(snapshot)?;
        Ok(RepoAuthorityRetirementProof {
            _authority_lock: authority_lock,
            repo_id,
            generation: snapshot.generation(),
        })
    }

    fn validate_retired_slot(
        &self,
        snapshot: &RepoAuthorityRemovalSnapshot,
    ) -> Result<(), LocalAuthorityError> {
        let repo_id = snapshot.repo_id();
        let generation = snapshot.generation();
        let slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        match slots.get(&repo_id) {
            Some(RepoAuthoritySlot::Retired {
                prior_generation,
                expected_lock_identity,
                removed_database_identity,
            }) if *prior_generation == generation
                && expected_lock_identity == snapshot.authority_lock()
                && removed_database_identity == snapshot.database() =>
            {
                Ok(())
            }
            None => Ok(()),
            Some(slot) => Err(LocalAuthorityError::StaleGeneration {
                repo_id,
                expected: generation,
                actual: slot_generation(Some(slot)).unwrap_or(0),
            }),
        }
    }
}
