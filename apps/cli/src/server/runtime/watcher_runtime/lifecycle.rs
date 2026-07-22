//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Nonblocking lifecycle reservation and handle extraction.

use super::error::WatcherLifecycleError;
use super::slot::WatcherMountSnapshot;
use super::slot::{MountSlot, RepoMountState, SlotTransitionError};
use super::supervisor::WatcherSupervisor;
use deve_core::models::RepoId;
use deve_core::sync::watcher::RepoWatcherHandle;
#[cfg(test)]
use deve_core::sync::watcher::{WatcherFailure, WatcherRefresh};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::Ordering;

#[must_use = "a watcher lifecycle reservation must be finalized"]
pub(crate) struct WatcherMountReservation {
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
    pub(super) target: Arc<MountSlot>,
    pub(super) previous: Option<Arc<MountSlot>>,
    pub(super) previous_state: Option<RepoMountState>,
    #[allow(dead_code)] // R4 rollback restores the exact prior mount slot.
    pub(super) previous_expected_handle: bool,
}

impl WatcherMountReservation {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn previous_generation(&self) -> Option<u64> {
        self.previous.as_ref().map(|slot| slot.generation())
    }
}

impl WatcherSupervisor {
    pub(crate) fn reserve_new(
        &self,
        repo_id: RepoId,
    ) -> Result<WatcherMountReservation, WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned.shutting_down {
            return Err(WatcherLifecycleError::Coordination(
                "watcher supervisor is shutting down",
            ));
        }
        let mut slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        if let Some(slot) = slots.get(&repo_id) {
            return Err(WatcherLifecycleError::AlreadyReserved {
                repo_id,
                generation: slot.generation(),
            });
        }
        let generation = 1;
        let target = Arc::new(MountSlot::transitioning(repo_id, generation));
        slots.insert(repo_id, target.clone());
        Ok(WatcherMountReservation {
            repo_id,
            generation,
            target,
            previous: None,
            previous_state: None,
            previous_expected_handle: false,
        })
    }

    #[allow(dead_code)] // R4 ownership-aware remove consumes this capability.
    pub(crate) fn reserve_existing(
        &self,
        repo_id: RepoId,
    ) -> Result<WatcherMountReservation, WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned.shutting_down {
            return Err(WatcherLifecycleError::Coordination(
                "watcher supervisor is shutting down",
            ));
        }
        let previous = {
            let slots = self.slots.lock().map_err(|_| {
                WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
            })?;
            slots
                .get(&repo_id)
                .cloned()
                .ok_or(WatcherLifecycleError::Missing(repo_id))?
        };
        let generation = previous
            .generation()
            .checked_add(1)
            .ok_or(WatcherLifecycleError::GenerationExhausted(repo_id))?;
        let previous_expected_handle = owned
            .handles
            .get(&repo_id)
            .is_some_and(|entry| entry.generation == previous.generation());
        let previous_state = match previous.begin_lifecycle_transition() {
            Ok(state) => state,
            Err(SlotTransitionError::Busy(state)) => {
                return Err(WatcherLifecycleError::Busy {
                    repo_id,
                    generation: previous.generation(),
                    state,
                });
            }
            Err(SlotTransitionError::Coordination) => {
                return Err(WatcherLifecycleError::Coordination(
                    "watcher refresh route poisoned while reserving lifecycle transition",
                ));
            }
        };
        let target = Arc::new(MountSlot::transitioning(repo_id, generation));
        {
            let mut slots = self.slots.lock().map_err(|_| {
                WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
            })?;
            let is_current = slots
                .get(&repo_id)
                .is_some_and(|slot| Arc::ptr_eq(slot, &previous));
            if !is_current {
                return Err(WatcherLifecycleError::StaleReservation {
                    repo_id,
                    generation,
                });
            }
            slots.insert(repo_id, target.clone());
        }
        Ok(WatcherMountReservation {
            repo_id,
            generation,
            target,
            previous: Some(previous),
            previous_state: Some(previous_state),
            previous_expected_handle: previous_expected_handle
                && previous_state != RepoMountState::Stopped,
        })
    }

    pub(crate) fn cancel_unstarted(
        &self,
        reservation: WatcherMountReservation,
    ) -> Result<(), WatcherLifecycleError> {
        self.validate_current(&reservation)?;
        if self.has_handle(reservation.repo_id)? && reservation.previous.is_none() {
            return Err(WatcherLifecycleError::HandleStillOwned {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            });
        }
        let mut slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        let is_current = slots
            .get(&reservation.repo_id)
            .is_some_and(|slot| Arc::ptr_eq(slot, &reservation.target));
        if !is_current {
            return Err(WatcherLifecycleError::StaleReservation {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            });
        }
        reservation.target.drop_deferred();
        match (reservation.previous, reservation.previous_state) {
            (Some(previous), Some(state)) => {
                previous.restore_after_cancel(state, &self.publisher)?;
                slots.insert(reservation.repo_id, previous);
            }
            (None, None) => {
                slots.remove(&reservation.repo_id);
            }
            _ => {
                return Err(WatcherLifecycleError::Coordination(
                    "watcher reservation previous state is incomplete",
                ));
            }
        }
        Ok(())
    }

    #[allow(dead_code)] // R4 ownership-aware remove consumes this capability.
    pub(crate) fn shutdown_reserved(
        &self,
        reservation: &WatcherMountReservation,
    ) -> Result<(), WatcherLifecycleError> {
        self.validate_current(reservation)?;
        let Some(previous) = reservation.previous.as_ref() else {
            return Ok(());
        };
        let handle = self.take_handle(reservation.repo_id, previous.generation())?;
        let Some(handle) = handle else {
            previous.mark_stopped_preserving_refresh();
            if reservation.previous_expected_handle {
                return Err(WatcherLifecycleError::Coordination(
                    "watcher handle missing during reserved shutdown",
                ));
            }
            return Ok(());
        };

        let result = handle.shutdown();
        previous.mark_stopped_preserving_refresh();
        match result {
            Err(failure) => Err(WatcherLifecycleError::Shutdown {
                repo_id: reservation.repo_id,
                failure,
            }),
            Ok(()) => {
                #[cfg(test)]
                if self
                    .fail_next_shutdown_after_cleanup
                    .swap(false, Ordering::AcqRel)
                {
                    return Err(WatcherLifecycleError::Shutdown {
                        repo_id: reservation.repo_id,
                        failure: WatcherFailure::new(
                            deve_core::sync::watcher::WatcherFailurePhase::Shutdown,
                            deve_core::sync::watcher::WatcherFailureKind::Repository,
                            "injected watcher final reconcile failure",
                        ),
                    });
                }
                Ok(())
            }
        }
    }

    #[allow(dead_code)] // R4 ownership-aware remove consumes this capability.
    pub(crate) fn finalize_removed(
        &self,
        reservation: WatcherMountReservation,
    ) -> Result<(), WatcherLifecycleError> {
        self.validate_current(&reservation)?;
        if self.has_handle(reservation.repo_id)? {
            return Err(WatcherLifecycleError::HandleStillOwned {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            });
        }
        if let Some(previous) = &reservation.previous {
            previous.drop_deferred();
        }
        reservation.target.drop_deferred();
        reservation.target.mark_stopped_preserving_refresh();
        let mut slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        let is_current = slots
            .get(&reservation.repo_id)
            .is_some_and(|slot| Arc::ptr_eq(slot, &reservation.target));
        if !is_current {
            return Err(WatcherLifecycleError::StaleReservation {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            });
        }
        slots.remove(&reservation.repo_id);
        Ok(())
    }

    pub(crate) fn cleanup_removed_repo_runtime(
        &self,
        repo_id: RepoId,
    ) -> Result<bool, WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned.handles.contains_key(&repo_id) {
            return Err(WatcherLifecycleError::HandleStillOwned {
                repo_id,
                generation: owned
                    .handles
                    .get(&repo_id)
                    .map(|entry| entry.generation)
                    .unwrap_or(0),
            });
        }
        let mut slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        let Some(slot) = slots.remove(&repo_id) else {
            return Ok(false);
        };
        slot.drop_deferred();
        slot.mark_stopped_preserving_refresh();
        Ok(true)
    }

    pub(crate) fn removed_repo_runtime_is_absent(
        &self,
        repo_id: RepoId,
    ) -> Result<bool, WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned.handles.contains_key(&repo_id) {
            return Ok(false);
        }
        let slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        Ok(!slots.contains_key(&repo_id))
    }

    #[cfg(test)]
    pub(crate) fn discard_deferred(
        &self,
        reservation: &WatcherMountReservation,
    ) -> Result<(), WatcherLifecycleError> {
        self.validate_current(reservation)?;
        if let Some(previous) = &reservation.previous {
            previous.drop_deferred();
        }
        reservation.target.drop_deferred();
        Ok(())
    }

    pub(crate) fn snapshot(
        &self,
        repo_id: RepoId,
    ) -> Result<WatcherMountSnapshot, WatcherLifecycleError> {
        self.slots
            .lock()
            .map_err(|_| {
                WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
            })?
            .get(&repo_id)
            .map(|slot| slot.snapshot())
            .ok_or(WatcherLifecycleError::Missing(repo_id))
    }

    pub(crate) fn mounted_generation(&self, repo_id: RepoId) -> Result<u64, WatcherLifecycleError> {
        let snapshot = self.snapshot(repo_id)?;
        if snapshot.state() == RepoMountState::Mounted {
            Ok(snapshot.generation())
        } else {
            Err(WatcherLifecycleError::Busy {
                repo_id,
                generation: snapshot.generation(),
                state: snapshot.state(),
            })
        }
    }

    pub(super) fn ensure_accepting(&self) -> Result<(), WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned.shutting_down {
            Err(WatcherLifecycleError::Coordination(
                "watcher supervisor is shutting down",
            ))
        } else {
            Ok(())
        }
    }

    pub(super) fn validate_current(
        &self,
        reservation: &WatcherMountReservation,
    ) -> Result<(), WatcherLifecycleError> {
        let slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        let current = slots
            .get(&reservation.repo_id)
            .is_some_and(|slot| Arc::ptr_eq(slot, &reservation.target));
        if current && reservation.target.generation() == reservation.generation {
            Ok(())
        } else {
            Err(WatcherLifecycleError::StaleReservation {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            })
        }
    }

    pub(super) fn has_handle(&self, repo_id: RepoId) -> Result<bool, WatcherLifecycleError> {
        self.owned
            .lock()
            .map(|owned| owned.handles.contains_key(&repo_id))
            .map_err(|_| {
                WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
            })
    }

    pub(super) fn take_handle(
        &self,
        repo_id: RepoId,
        generation: u64,
    ) -> Result<Option<RepoWatcherHandle>, WatcherLifecycleError> {
        let mut owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        let Some(entry) = owned.handles.get(&repo_id) else {
            return Ok(None);
        };
        if entry.generation != generation {
            return Err(WatcherLifecycleError::StaleReservation {
                repo_id,
                generation,
            });
        }
        let entry = owned
            .handles
            .remove(&repo_id)
            .expect("handle presence checked under the same owner lock");
        owned
            .start_order
            .retain(|key| !(key.repo_id == repo_id && key.generation == generation));
        Ok(Some(entry.handle))
    }

    #[cfg(test)]
    pub(crate) fn route_refresh_for_test(
        &self,
        repo_id: RepoId,
        generation: u64,
        refresh: WatcherRefresh,
    ) -> Result<(), WatcherLifecycleError> {
        let slot = self.slot_for_generation(repo_id, generation)?;
        slot.route_refresh(refresh, &self.publisher);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fail_for_test(
        &self,
        repo_id: RepoId,
        generation: u64,
        failure: WatcherFailure,
    ) -> Result<(), WatcherLifecycleError> {
        let slot = self.slot_for_generation(repo_id, generation)?;
        slot.fail(failure);
        Ok(())
    }

    #[cfg(test)]
    fn slot_for_generation(
        &self,
        repo_id: RepoId,
        generation: u64,
    ) -> Result<Arc<MountSlot>, WatcherLifecycleError> {
        let slot = self
            .slots
            .lock()
            .map_err(|_| {
                WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
            })?
            .get(&repo_id)
            .cloned()
            .ok_or(WatcherLifecycleError::Missing(repo_id))?;
        if slot.generation() == generation {
            Ok(slot)
        } else {
            Err(WatcherLifecycleError::StaleReservation {
                repo_id,
                generation,
            })
        }
    }
}
