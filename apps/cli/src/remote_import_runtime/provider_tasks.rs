//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 06_backup#remote-import-runtime-boundary
//!
//! Repo-scoped provider task generations and removal quiescence. This is
//! process-local coordination only; durable Remote Import state stays in core.

use deve_core::ledger::{CatalogMembershipError, CatalogMembershipRuntime, CatalogMembershipToken};
use deve_core::models::RepoId;
use deve_core::remote_import::RemoteImportSessionId;
use std::collections::HashMap;
use std::sync::{Condvar, Mutex};

#[derive(Debug)]
pub(super) enum ProviderTaskError {
    Busy,
    Membership(CatalogMembershipError),
    Coordination,
}

#[derive(Default)]
struct ProviderTaskSlot {
    generation: u64,
    active: bool,
    quiescing: bool,
    session_id: Option<RemoteImportSessionId>,
}

#[derive(Default)]
pub(super) struct ProviderTaskRuntime {
    slots: Mutex<HashMap<RepoId, ProviderTaskSlot>>,
    idle: Condvar,
}

#[derive(Debug)]
pub(crate) struct ProviderQuiesceToken {
    repo_id: RepoId,
    generation: u64,
}

pub(super) struct ProviderTaskLease<'a> {
    runtime: &'a ProviderTaskRuntime,
    repo_id: RepoId,
    generation: u64,
    session_id: Option<RemoteImportSessionId>,
}

impl ProviderTaskRuntime {
    pub(super) fn acquire(
        &self,
        repo_id: RepoId,
    ) -> Result<ProviderTaskLease<'_>, ProviderTaskError> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let slot = slots.entry(repo_id).or_default();
        if slot.active || slot.quiescing {
            return Err(ProviderTaskError::Busy);
        }
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or(ProviderTaskError::Coordination)?;
        slot.active = true;
        slot.session_id = None;
        Ok(ProviderTaskLease {
            runtime: self,
            repo_id,
            generation: slot.generation,
            session_id: None,
        })
    }

    pub(super) fn quiesce(
        &self,
        repo_id: RepoId,
    ) -> Result<ProviderQuiesceToken, ProviderTaskError> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let slot = slots.entry(repo_id).or_default();
        slot.quiescing = true;
        let generation = slot.generation;
        while slots
            .get(&repo_id)
            .is_some_and(|slot| slot.active && slot.generation == generation)
        {
            slots = self
                .idle
                .wait(slots)
                .map_err(|_| ProviderTaskError::Coordination)?;
        }
        Ok(ProviderQuiesceToken {
            repo_id,
            generation,
        })
    }

    pub(super) fn resume(&self, token: &ProviderQuiesceToken) -> Result<(), ProviderTaskError> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let slot = slots
            .get_mut(&token.repo_id)
            .ok_or(ProviderTaskError::Coordination)?;
        if slot.generation != token.generation || slot.active || !slot.quiescing {
            return Err(ProviderTaskError::Coordination);
        }
        slot.quiescing = false;
        self.idle.notify_all();
        Ok(())
    }

    pub(super) fn finish(&self, token: ProviderQuiesceToken) -> Result<(), ProviderTaskError> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let exact = slots.get(&token.repo_id).is_some_and(|slot| {
            slot.generation == token.generation && !slot.active && slot.quiescing
        });
        if !exact {
            return Err(ProviderTaskError::Coordination);
        }
        slots.remove(&token.repo_id);
        Ok(())
    }
}

impl ProviderTaskLease<'_> {
    pub(super) fn bind_session(
        &mut self,
        session_id: RemoteImportSessionId,
    ) -> Result<(), ProviderTaskError> {
        let mut slots = self
            .runtime
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let slot = slots
            .get_mut(&self.repo_id)
            .ok_or(ProviderTaskError::Coordination)?;
        if slot.generation != self.generation || !slot.active || slot.session_id.is_some() {
            return Err(ProviderTaskError::Coordination);
        }
        slot.session_id = Some(session_id);
        self.session_id = Some(session_id);
        Ok(())
    }

    pub(super) fn revalidate_completion(
        &self,
        membership_runtime: &CatalogMembershipRuntime,
        membership: &CatalogMembershipToken,
        session_id: RemoteImportSessionId,
    ) -> Result<(), ProviderTaskError> {
        membership_runtime
            .revalidate(membership)
            .map_err(ProviderTaskError::Membership)?;
        let slots = self
            .runtime
            .slots
            .lock()
            .map_err(|_| ProviderTaskError::Coordination)?;
        let exact = slots.get(&self.repo_id).is_some_and(|slot| {
            slot.generation == self.generation
                && slot.active
                && !slot.quiescing
                && slot.session_id == Some(session_id)
                && self.session_id == Some(session_id)
        });
        if exact {
            Ok(())
        } else {
            Err(ProviderTaskError::Membership(
                CatalogMembershipError::NotMember(self.repo_id),
            ))
        }
    }
}

impl Drop for ProviderTaskLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut slots) = self.runtime.slots.lock()
            && let Some(slot) = slots.get_mut(&self.repo_id)
            && slot.generation == self.generation
        {
            slot.active = false;
            slot.session_id = None;
            self.runtime.idle.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiesce_blocks_new_provider_generation_until_resume() {
        let runtime = ProviderTaskRuntime::default();
        let repo_id = RepoId::new_v4();
        let token = runtime.quiesce(repo_id).expect("quiesce");
        assert!(matches!(
            runtime.acquire(repo_id),
            Err(ProviderTaskError::Busy)
        ));
        runtime.resume(&token).expect("resume");
        drop(runtime.acquire(repo_id).expect("acquire after resume"));
    }

    #[test]
    fn finish_removes_exact_quiesced_slot() {
        let runtime = ProviderTaskRuntime::default();
        let repo_id = RepoId::new_v4();
        let token = runtime.quiesce(repo_id).expect("quiesce");
        runtime.finish(token).expect("finish");
        drop(runtime.acquire(repo_id).expect("new provider generation"));
    }
}
