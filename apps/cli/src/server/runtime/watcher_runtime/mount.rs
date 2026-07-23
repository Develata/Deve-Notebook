//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Reservation-bound watcher start and mount finalization.

use super::error::WatcherLifecycleError;
use super::lifecycle::WatcherMountReservation;
use super::slot::{MountSlot, RepoMountState, WatcherMountSnapshot};
use super::supervisor::{HandleKey, OwnedHandle, WatcherSupervisor, coordination_failure};
use deve_core::models::RepoId;
use deve_core::sync::watcher::{RepoWatcherHandle, RepoWatcherStart, WatcherFailure};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::Ordering;

impl WatcherSupervisor {
    pub(crate) fn start_reserved(
        &self,
        reservation: &WatcherMountReservation,
        start: RepoWatcherStart,
    ) -> Result<(), WatcherLifecycleError> {
        self.start_reserved_inner(reservation, start)
    }

    pub(crate) fn finalize_mounted(
        &self,
        reservation: &WatcherMountReservation,
    ) -> Result<WatcherMountSnapshot, WatcherLifecycleError> {
        self.validate_current(reservation)?;
        self.ensure_handle_generation(reservation.repo_id, reservation.generation)?;
        let previous_refresh = reservation
            .previous
            .as_ref()
            .map(|slot| slot.take_deferred())
            .transpose()
            .map_err(|_| refresh_coordination_error(reservation.repo_id))?
            .flatten();
        reservation
            .target
            .merge_deferred(previous_refresh)
            .map_err(|_| refresh_coordination_error(reservation.repo_id))?;
        if let Err(failure) = reservation.target.mark_mounted(&self.publisher) {
            let cleanup = self
                .take_handle(reservation.repo_id, reservation.generation)?
                .and_then(|handle| handle.shutdown().err());
            return Err(WatcherLifecycleError::FailedBeforeMounted {
                repo_id: reservation.repo_id,
                failure: Box::new(failure),
                cleanup: cleanup.map(Box::new),
            });
        }
        Ok(reservation.target.snapshot())
    }

    pub(crate) fn finalize_failed(
        &self,
        reservation: &WatcherMountReservation,
        failure: WatcherFailure,
    ) -> Result<WatcherMountSnapshot, WatcherLifecycleError> {
        self.validate_current(reservation)?;
        if let Some(previous) = &reservation.previous {
            previous.drop_deferred();
        }
        reservation.target.mark_failed_and_drop(failure.clone());
        let cleanup = self
            .take_handle(reservation.repo_id, reservation.generation)?
            .and_then(|handle| handle.shutdown().err());
        if cleanup.is_some() {
            return Err(WatcherLifecycleError::FailedBeforeMounted {
                repo_id: reservation.repo_id,
                failure: Box::new(failure),
                cleanup: cleanup.map(Box::new),
            });
        }
        Ok(reservation.target.snapshot())
    }

    pub(super) fn reserve_bootstrap(
        &self,
        repo_id: RepoId,
        generation: u64,
    ) -> Result<WatcherMountReservation, WatcherLifecycleError> {
        self.ensure_accepting()?;
        let mut slots = self.slots.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor slot registry poisoned")
        })?;
        if slots.contains_key(&repo_id) {
            return Err(WatcherLifecycleError::AlreadyReserved {
                repo_id,
                generation,
            });
        }
        let target = Arc::new(MountSlot::starting(repo_id, generation));
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

    pub(super) fn start_reserved_inner(
        &self,
        reservation: &WatcherMountReservation,
        start: RepoWatcherStart,
    ) -> Result<(), WatcherLifecycleError> {
        self.validate_current(reservation)?;
        if start.repo_id() != reservation.repo_id || start.generation() != reservation.generation {
            return Err(WatcherLifecycleError::StartIdentity {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
                actual_repo_id: start.repo_id(),
                actual_generation: start.generation(),
            });
        }
        #[cfg(test)]
        if self.fail_next_start.swap(false, Ordering::AcqRel) {
            return Err(WatcherLifecycleError::FailedBeforeMounted {
                repo_id: reservation.repo_id,
                failure: Box::new(coordination_failure("injected watcher start failure")),
                cleanup: None,
            });
        }
        let refresh_slot = reservation.target.clone();
        let refresh_publisher = self.publisher.clone();
        let failure_slot = reservation.target.clone();
        let start = start
            .with_refresh(Arc::new(move |refresh| {
                refresh_slot.route_refresh(refresh, &refresh_publisher);
            }))
            .with_failure_callback(Arc::new(move |failure| {
                failure_slot.fail(failure);
            }));

        let handle = RepoWatcherHandle::start(start).map_err(|source| {
            reservation.target.fail(source.failure().clone());
            WatcherLifecycleError::Start {
                repo_id: reservation.repo_id,
                source,
            }
        })?;
        if !matches!(
            reservation.target.state(),
            RepoMountState::Starting | RepoMountState::Transitioning
        ) {
            let failure = reservation.target.recorded_failure().unwrap_or_else(|| {
                coordination_failure("watcher failed before supervisor handle ownership")
            });
            let cleanup = handle.shutdown().err();
            return Err(WatcherLifecycleError::FailedBeforeMounted {
                repo_id: reservation.repo_id,
                failure: Box::new(failure),
                cleanup: cleanup.map(Box::new),
            });
        }
        {
            let mut owned = match self.owned.lock() {
                Ok(owned) => owned,
                Err(_) => {
                    let cleanup = handle.shutdown().err();
                    reservation.target.fail(coordination_failure(
                        "watcher supervisor owner registry poisoned",
                    ));
                    return Err(WatcherLifecycleError::HostCoordination {
                        detail: "watcher supervisor owner registry poisoned",
                        cleanup: cleanup.map(Box::new),
                    });
                }
            };
            if owned.shutting_down || owned.handles.contains_key(&reservation.repo_id) {
                drop(owned);
                let cleanup = handle.shutdown().err();
                let failure = coordination_failure(
                    "watcher supervisor rejected handle ownership during lifecycle start",
                );
                reservation.target.fail(failure.clone());
                return Err(WatcherLifecycleError::FailedBeforeMounted {
                    repo_id: reservation.repo_id,
                    failure: Box::new(failure),
                    cleanup: cleanup.map(Box::new),
                });
            }
            owned.start_order.push(HandleKey {
                repo_id: reservation.repo_id,
                generation: reservation.generation,
            });
            owned.handles.insert(
                reservation.repo_id,
                OwnedHandle {
                    generation: reservation.generation,
                    handle,
                },
            );
        }
        if reservation.target.state() == RepoMountState::Failed {
            let failure = reservation.target.recorded_failure().unwrap_or_else(|| {
                coordination_failure("watcher failed before mount finalization")
            });
            let cleanup = self
                .take_handle(reservation.repo_id, reservation.generation)?
                .and_then(|handle| handle.shutdown().err());
            return Err(WatcherLifecycleError::FailedBeforeMounted {
                repo_id: reservation.repo_id,
                failure: Box::new(failure),
                cleanup: cleanup.map(Box::new),
            });
        }
        Ok(())
    }

    fn ensure_handle_generation(
        &self,
        repo_id: RepoId,
        generation: u64,
    ) -> Result<(), WatcherLifecycleError> {
        let owned = self.owned.lock().map_err(|_| {
            WatcherLifecycleError::Coordination("watcher supervisor owner registry poisoned")
        })?;
        if owned
            .handles
            .get(&repo_id)
            .is_some_and(|owned| owned.generation == generation)
        {
            Ok(())
        } else {
            Err(WatcherLifecycleError::HandleStillOwned {
                repo_id,
                generation,
            })
        }
    }
}

fn refresh_coordination_error(repo_id: RepoId) -> WatcherLifecycleError {
    WatcherLifecycleError::FailedBeforeMounted {
        repo_id,
        failure: Box::new(coordination_failure(
            "watcher refresh route poisoned during lifecycle finalization",
        )),
        cleanup: None,
    }
}
