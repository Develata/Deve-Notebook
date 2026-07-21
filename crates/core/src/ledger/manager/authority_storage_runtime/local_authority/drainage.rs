//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Lease-drain transition for the bounded pre-commit removal cut.

use super::{
    LocalAuthorityError, LocalAuthorityRuntime, RepoAuthorityQuiesceGuard, RepoAuthorityResources,
    RepoAuthoritySlot,
};
use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

impl LocalAuthorityRuntime {
    #[allow(dead_code)] // Called by the approved R3/R4 removal coordinator.
    pub(crate) fn quiesce(
        &self,
        repo_id: RepoId,
        expected_generation: u64,
    ) -> Result<RepoAuthorityQuiesceGuard, LocalAuthorityError> {
        self.quiesce_with_timeout(repo_id, expected_generation, Duration::from_secs(30))
    }

    #[cfg(test)]
    pub(crate) fn quiesce_for_test(
        &self,
        repo_id: RepoId,
        expected_generation: u64,
        timeout: Duration,
    ) -> Result<RepoAuthorityQuiesceGuard, LocalAuthorityError> {
        self.quiesce_with_timeout(repo_id, expected_generation, timeout)
    }

    fn quiesce_with_timeout(
        &self,
        repo_id: RepoId,
        expected_generation: u64,
        timeout: Duration,
    ) -> Result<RepoAuthorityQuiesceGuard, LocalAuthorityError> {
        let deadline = Instant::now() + timeout;
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let resources = match slots.remove(&repo_id) {
            Some(RepoAuthoritySlot::Active {
                generation,
                resources,
                leases,
            }) if generation == expected_generation => {
                slots.insert(
                    repo_id,
                    RepoAuthoritySlot::Quiescing {
                        generation,
                        resources: Some(resources.clone()),
                        leases,
                    },
                );
                resources
            }
            Some(RepoAuthoritySlot::Active {
                generation,
                resources,
                leases,
            }) => {
                slots.insert(
                    repo_id,
                    RepoAuthoritySlot::Active {
                        generation,
                        resources,
                        leases,
                    },
                );
                return Err(LocalAuthorityError::StaleGeneration {
                    repo_id,
                    expected: expected_generation,
                    actual: generation,
                });
            }
            Some(slot) => {
                slots.insert(repo_id, slot);
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            None => return Err(LocalAuthorityError::Retired(repo_id)),
        };
        loop {
            let leases = match slots.get(&repo_id) {
                Some(RepoAuthoritySlot::Quiescing {
                    generation,
                    resources: Some(slot_resources),
                    leases,
                }) if *generation == expected_generation
                    && Arc::ptr_eq(slot_resources, &resources) =>
                {
                    *leases
                }
                _ => {
                    return Err(LocalAuthorityError::Invariant(format!(
                        "RepoId {repo_id} quiescing slot changed while draining leases"
                    )));
                }
            };
            if leases == 0 {
                break;
            }
            let now = Instant::now();
            if now >= deadline {
                restore_active_after_drain_timeout(
                    &mut slots,
                    repo_id,
                    expected_generation,
                    &resources,
                )?;
                return Err(LocalAuthorityError::DrainTimeout(repo_id));
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, _) = self
                .inner
                .lease_released
                .wait_timeout(slots, remaining)
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            slots = next;
        }
        Ok(RepoAuthorityQuiesceGuard {
            inner: self.inner.clone(),
            resources: Some(resources),
            repo_id,
            generation: expected_generation,
            settled: false,
        })
    }
}

fn restore_active_after_drain_timeout(
    slots: &mut HashMap<RepoId, RepoAuthoritySlot>,
    repo_id: RepoId,
    generation: u64,
    expected_resources: &Arc<RepoAuthorityResources>,
) -> Result<(), LocalAuthorityError> {
    let Some(slot) = slots.remove(&repo_id) else {
        return Err(LocalAuthorityError::Invariant(format!(
            "RepoId {repo_id} quiescing slot disappeared during timeout rollback"
        )));
    };
    let RepoAuthoritySlot::Quiescing {
        generation: actual_generation,
        resources: Some(resources),
        leases,
    } = slot
    else {
        slots.insert(repo_id, slot);
        return Err(LocalAuthorityError::Invariant(format!(
            "RepoId {repo_id} is not rollback-capable after drain timeout"
        )));
    };
    if actual_generation != generation || !Arc::ptr_eq(&resources, expected_resources) {
        slots.insert(
            repo_id,
            RepoAuthoritySlot::Quiescing {
                generation: actual_generation,
                resources: Some(resources),
                leases,
            },
        );
        return Err(LocalAuthorityError::StaleGeneration {
            repo_id,
            expected: generation,
            actual: actual_generation,
        });
    }
    slots.insert(
        repo_id,
        RepoAuthoritySlot::Active {
            generation,
            resources,
            leases,
        },
    );
    Ok(())
}
