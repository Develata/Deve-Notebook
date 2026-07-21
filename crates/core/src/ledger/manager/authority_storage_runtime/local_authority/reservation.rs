//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! Panic-safe ownership of an exact Opening slot reservation.

use super::{LocalAuthorityError, LocalAuthorityInner, RepoAuthoritySlot};
use crate::models::RepoId;
use std::sync::Arc;
use uuid::Uuid;

pub(super) struct OpeningReservation {
    inner: Arc<LocalAuthorityInner>,
    repo_id: RepoId,
    reservation_id: Uuid,
    repair_generation_on_drop: Option<u64>,
    settled: bool,
}

impl OpeningReservation {
    pub(super) fn new(
        inner: &Arc<LocalAuthorityInner>,
        repo_id: RepoId,
        reservation_id: Uuid,
    ) -> Self {
        Self {
            inner: inner.clone(),
            repo_id,
            reservation_id,
            repair_generation_on_drop: None,
            settled: false,
        }
    }

    pub(super) fn require_repair_on_drop(&mut self) {
        self.repair_generation_on_drop = Some(1);
    }

    pub(super) fn restore_repair_on_drop(&mut self, generation: u64) {
        self.repair_generation_on_drop = Some(generation);
    }

    pub(super) fn settle_after_transition(&mut self) {
        self.settled = true;
    }

    pub(super) fn remove(mut self) -> Result<(), LocalAuthorityError> {
        self.transition_opening(None)?;
        self.settled = true;
        Ok(())
    }

    pub(super) fn restore_repair(mut self, generation: u64) -> Result<(), LocalAuthorityError> {
        self.transition_opening(Some(generation))?;
        self.settled = true;
        Ok(())
    }

    fn transition_opening(
        &self,
        repair_generation: Option<u64>,
    ) -> Result<(), LocalAuthorityError> {
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if !matches!(
            slots.get(&self.repo_id),
            Some(RepoAuthoritySlot::Opening {
                reservation_id: current,
            }) if *current == self.reservation_id
        ) {
            return Err(LocalAuthorityError::Invariant(format!(
                "opening reservation changed for RepoId {}",
                self.repo_id
            )));
        }
        if let Some(generation) = repair_generation {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::RepairRequired { generation },
            );
        } else {
            slots.remove(&self.repo_id);
        }
        Ok(())
    }
}

impl Drop for OpeningReservation {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        if let Err(error) = self.transition_opening(self.repair_generation_on_drop) {
            tracing::error!(repo_id = %self.repo_id, %error, "failed to settle dropped local authority opening reservation");
        }
    }
}
