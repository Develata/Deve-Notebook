//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Process-local catalog membership readiness. Durable membership remains in
//! the repository catalog; this runtime only invalidates stale host bindings.

use crate::models::RepoId;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use uuid::Uuid;

mod capability;
mod registry;

pub use capability::{
    CatalogMembershipError, CatalogMembershipGeneration, CatalogMembershipToken,
    RepoCatalogCutAuthority, RepoCatalogCutPermit,
};

/// Cloneable capability for the single process-local catalog membership
/// authority owned by a `RepoManager` composition root.
#[derive(Clone)]
pub struct CatalogMembershipRuntime {
    inner: Arc<CatalogMembershipInner>,
}

struct CatalogMembershipInner {
    runtime_instance: Uuid,
    cut: Mutex<()>,
    cut_authority: Mutex<Option<Uuid>>,
    state: RwLock<CatalogMembershipState>,
}

#[derive(Default)]
struct CatalogMembershipState {
    seeded: bool,
    slots: HashMap<RepoId, CatalogMembershipSlot>,
}

#[derive(Debug, Clone, Copy)]
struct CatalogMembershipSlot {
    generation: CatalogMembershipGeneration,
    phase: CatalogMembershipPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatalogMembershipPhase {
    Normal,
    Revoking(Uuid),
    Removed,
}

#[derive(Debug, Clone)]
pub(super) struct CatalogMembershipRevocation {
    runtime_instance: Uuid,
    repo_id: RepoId,
    old_generation: CatalogMembershipGeneration,
    blocked_generation: CatalogMembershipGeneration,
    reservation_id: Uuid,
}

impl fmt::Debug for CatalogMembershipRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogMembershipRuntime")
            .finish_non_exhaustive()
    }
}

impl CatalogMembershipRuntime {
    #[cfg(test)]
    fn isolated() -> Self {
        Self {
            inner: Arc::new(CatalogMembershipInner {
                runtime_instance: Uuid::new_v4(),
                cut: Mutex::new(()),
                cut_authority: Mutex::new(None),
                state: RwLock::new(CatalogMembershipState::default()),
            }),
        }
    }

    /// Seeds the exact bootstrap catalog. Repeating the same current member
    /// set is idempotent; duplicate or different sets fail closed.
    pub(crate) fn seed(
        &self,
        repo_ids: impl IntoIterator<Item = RepoId>,
    ) -> Result<(), CatalogMembershipError> {
        let mut unique = HashSet::new();
        let mut ordered = Vec::new();
        for repo_id in repo_ids {
            if !unique.insert(repo_id) {
                return Err(CatalogMembershipError::DuplicateSeed(repo_id));
            }
            ordered.push(repo_id);
        }
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        if state.seeded {
            let current = state
                .slots
                .iter()
                .filter_map(|(repo_id, slot)| {
                    (slot.phase == CatalogMembershipPhase::Normal).then_some(*repo_id)
                })
                .collect::<HashSet<_>>();
            return if current == unique {
                Ok(())
            } else {
                Err(CatalogMembershipError::SeedDrift)
            };
        }
        state.slots.extend(ordered.into_iter().map(|repo_id| {
            (
                repo_id,
                CatalogMembershipSlot {
                    generation: CatalogMembershipGeneration::INITIAL,
                    phase: CatalogMembershipPhase::Normal,
                },
            )
        }));
        state.seeded = true;
        Ok(())
    }

    pub fn issue(&self, repo_id: RepoId) -> Result<CatalogMembershipToken, CatalogMembershipError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        let slot = state
            .slots
            .get(&repo_id)
            .filter(|slot| slot.phase == CatalogMembershipPhase::Normal)
            .ok_or(CatalogMembershipError::NotMember(repo_id))?;
        Ok(self.token(repo_id, slot.generation))
    }

    pub fn revalidate(&self, token: &CatalogMembershipToken) -> Result<(), CatalogMembershipError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        self.validate_token(&state, token)
    }

    pub(super) fn validate_cut_permit(
        &self,
        permit: &RepoCatalogCutPermit,
        repo_id: RepoId,
    ) -> Result<(), CatalogMembershipError> {
        let active_authority = self
            .inner
            .cut_authority
            .lock()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        if permit.runtime_instance == self.inner.runtime_instance
            && active_authority.as_ref() == Some(&permit.authority_instance)
            && permit.repo_id == repo_id
        {
            Ok(())
        } else {
            Err(CatalogMembershipError::CutPermitMismatch(repo_id))
        }
    }

    #[cfg(test)]
    pub(super) fn cut_permit_for_test(&self, repo_id: RepoId) -> RepoCatalogCutPermit {
        let mut owner = self
            .inner
            .cut_authority
            .lock()
            .expect("test cut authority lock");
        let authority_instance = match *owner {
            Some(authority) => authority,
            None => {
                let authority = Uuid::new_v4();
                *owner = Some(authority);
                authority
            }
        };
        RepoCatalogCutPermit {
            runtime_instance: self.inner.runtime_instance,
            authority_instance,
            repo_id,
        }
    }

    pub(super) fn claim_cut_authority(
        &self,
    ) -> Result<RepoCatalogCutAuthority, CatalogMembershipError> {
        let authority_instance = Uuid::new_v4();
        let mut owner = self
            .inner
            .cut_authority
            .lock()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        if owner.is_some() {
            return Err(CatalogMembershipError::CutAuthorityAlreadyClaimed);
        }
        *owner = Some(authority_instance);
        Ok(RepoCatalogCutAuthority {
            runtime: self.clone(),
            authority_instance,
        })
    }

    /// Admits one newly-created or explicitly recovered repo after its durable
    /// catalog mutation commits. Existing membership is never silently reused.
    pub(super) fn admit_created(
        &self,
        repo_id: RepoId,
    ) -> Result<CatalogMembershipToken, CatalogMembershipError> {
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        let generation = match state.slots.get(&repo_id).copied() {
            Some(slot) if slot.phase == CatalogMembershipPhase::Normal => {
                return Err(CatalogMembershipError::AlreadyMember(repo_id));
            }
            Some(slot) => slot.generation.next(repo_id)?,
            None => CatalogMembershipGeneration::INITIAL,
        };
        state.slots.insert(
            repo_id,
            CatalogMembershipSlot {
                generation,
                phase: CatalogMembershipPhase::Normal,
            },
        );
        Ok(self.token(repo_id, generation))
    }

    /// Closes admission and rotates generation before a Removed record can
    /// become visible. The caller must later finalize or abort this exact
    /// reservation according to the typed publish phase.
    pub(super) fn begin_removal(
        &self,
        expected: &CatalogMembershipToken,
    ) -> Result<CatalogMembershipRevocation, CatalogMembershipError> {
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        self.validate_token(&state, expected)?;
        let next = expected.generation.next(expected.repo_id)?;
        let reservation_id = Uuid::new_v4();
        state.slots.insert(
            expected.repo_id,
            CatalogMembershipSlot {
                generation: next,
                phase: CatalogMembershipPhase::Revoking(reservation_id),
            },
        );
        Ok(CatalogMembershipRevocation {
            runtime_instance: self.inner.runtime_instance,
            repo_id: expected.repo_id,
            old_generation: expected.generation,
            blocked_generation: next,
            reservation_id,
        })
    }

    pub(super) fn finalize_removed(
        &self,
        revocation: &CatalogMembershipRevocation,
    ) -> Result<(), CatalogMembershipError> {
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        self.validate_revocation(&state, revocation)?;
        state.slots.insert(
            revocation.repo_id,
            CatalogMembershipSlot {
                generation: revocation.blocked_generation,
                phase: CatalogMembershipPhase::Removed,
            },
        );
        Ok(())
    }

    pub(super) fn abort_removal(
        &self,
        revocation: &CatalogMembershipRevocation,
    ) -> Result<CatalogMembershipToken, CatalogMembershipError> {
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        self.validate_revocation(&state, revocation)?;
        state.slots.insert(
            revocation.repo_id,
            CatalogMembershipSlot {
                generation: revocation.blocked_generation,
                phase: CatalogMembershipPhase::Normal,
            },
        );
        Ok(self.token(revocation.repo_id, revocation.blocked_generation))
    }

    pub(super) fn cut_guard(&self) -> Result<MutexGuard<'_, ()>, CatalogMembershipError> {
        self.inner
            .cut
            .lock()
            .map_err(|_| CatalogMembershipError::Poisoned)
    }

    pub(super) fn converge_removed(
        &self,
        expected: &CatalogMembershipToken,
    ) -> Result<bool, CatalogMembershipError> {
        let mut state = self
            .inner
            .state
            .write()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        ensure_seeded(&state)?;
        if expected.runtime_instance != self.inner.runtime_instance {
            return Err(CatalogMembershipError::RuntimeMismatch(expected.repo_id));
        }
        let Some(slot) = state.slots.get(&expected.repo_id).copied() else {
            return Ok(false);
        };
        let converged = slot.phase != CatalogMembershipPhase::Normal
            && expected
                .generation
                .0
                .checked_add(1)
                .is_some_and(|next| slot.generation.0 == next);
        if converged {
            state.slots.insert(
                expected.repo_id,
                CatalogMembershipSlot {
                    generation: slot.generation,
                    phase: CatalogMembershipPhase::Removed,
                },
            );
        }
        Ok(converged)
    }

    fn token(
        &self,
        repo_id: RepoId,
        generation: CatalogMembershipGeneration,
    ) -> CatalogMembershipToken {
        CatalogMembershipToken {
            runtime_instance: self.inner.runtime_instance,
            repo_id,
            generation,
        }
    }

    fn validate_token(
        &self,
        state: &CatalogMembershipState,
        token: &CatalogMembershipToken,
    ) -> Result<(), CatalogMembershipError> {
        if token.runtime_instance != self.inner.runtime_instance {
            return Err(CatalogMembershipError::RuntimeMismatch(token.repo_id));
        }
        let slot = state
            .slots
            .get(&token.repo_id)
            .ok_or(CatalogMembershipError::NotMember(token.repo_id))?;
        if slot.generation != token.generation {
            return Err(CatalogMembershipError::Stale {
                repo_id: token.repo_id,
                expected: token.generation,
                current: slot.generation,
            });
        }
        if slot.phase != CatalogMembershipPhase::Normal {
            return Err(CatalogMembershipError::NotMember(token.repo_id));
        }
        Ok(())
    }

    fn validate_revocation(
        &self,
        state: &CatalogMembershipState,
        revocation: &CatalogMembershipRevocation,
    ) -> Result<(), CatalogMembershipError> {
        if revocation.runtime_instance != self.inner.runtime_instance {
            return Err(CatalogMembershipError::RuntimeMismatch(revocation.repo_id));
        }
        let slot = state
            .slots
            .get(&revocation.repo_id)
            .ok_or(CatalogMembershipError::NotMember(revocation.repo_id))?;
        if slot.generation != revocation.blocked_generation {
            return Err(CatalogMembershipError::Stale {
                repo_id: revocation.repo_id,
                expected: revocation.blocked_generation,
                current: slot.generation,
            });
        }
        if slot.phase != CatalogMembershipPhase::Revoking(revocation.reservation_id) {
            return Err(CatalogMembershipError::NotMember(revocation.repo_id));
        }
        debug_assert!(revocation.blocked_generation > revocation.old_generation);
        Ok(())
    }
}

fn ensure_seeded(state: &CatalogMembershipState) -> Result<(), CatalogMembershipError> {
    if state.seeded {
        Ok(())
    } else {
        Err(CatalogMembershipError::NotSeeded)
    }
}

#[cfg(test)]
#[path = "membership/tests.rs"]
mod tests;
