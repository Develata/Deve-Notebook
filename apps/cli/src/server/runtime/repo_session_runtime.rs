//! plan_ref:
//!   - 04_repository#repo-health-and-repair
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 07_network#server-ws-runtime
//!
//! Process-local RepoBound registry and bounded removal invalidation fan-out.
//! Catalog authority remains in `CatalogMembershipRuntime`; this runtime only
//! owns per-connection projections and command delivery.

use crate::server::session::WsSession;
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::{CatalogMembershipRuntime, CatalogMembershipToken};
use deve_core::models::RepoId;
use deve_core::protocol::{RepoListEntry, ServerMessage};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Commands are not drained while a WS message handler is in flight, so this
/// must absorb the realistic burst (own settlement + concurrent removals)
/// before the fail-closed `try_send -> retire_session` backstop fires. Retire
/// is safe — settlements are durable and replayed via `GetLifecycle` on
/// reconnect — but should stay rare under legitimate concurrent load.
const SESSION_COMMAND_CAPACITY: usize = 8;

#[derive(Clone, Debug)]
pub(crate) struct FinalRepoListProjection {
    pub(crate) entries: Vec<RepoListEntry>,
}

#[derive(Clone)]
struct RepoBoundSnapshot {
    repo_id: RepoId,
    membership: CatalogMembershipToken,
    scope_nonce: u64,
}

struct SessionRegistration {
    binding: Option<RepoBoundSnapshot>,
    lifecycle_observers: HashMap<uuid::Uuid, RepoLifecycleSessionObserver>,
    commands: mpsc::Sender<RepoSessionCommand>,
    channel: DualChannel,
}

#[derive(Clone, Copy)]
struct RepoLifecycleSessionObserver {
    expected_scope_nonce: u64,
    switch_nonce: u64,
}

#[derive(Default)]
struct SessionRegistry {
    next_id: u64,
    sessions: HashMap<u64, SessionRegistration>,
}

pub(crate) struct RepoSessionRuntime {
    membership: CatalogMembershipRuntime,
    registry: Mutex<SessionRegistry>,
}

impl RepoSessionRuntime {
    pub(crate) fn new(membership: CatalogMembershipRuntime) -> Arc<Self> {
        Arc::new(Self {
            membership,
            registry: Mutex::new(SessionRegistry::default()),
        })
    }

    pub(crate) fn register(
        self: &Arc<Self>,
        channel: DualChannel,
    ) -> Result<(RepoSessionPermit, mpsc::Receiver<RepoSessionCommand>), RepoSessionRuntimeError>
    {
        let (commands, receiver) = mpsc::channel(SESSION_COMMAND_CAPACITY);
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        registry.next_id = registry
            .next_id
            .checked_add(1)
            .ok_or(RepoSessionRuntimeError::GenerationExhausted)?;
        let id = registry.next_id;
        registry.sessions.insert(
            id,
            SessionRegistration {
                binding: None,
                lifecycle_observers: HashMap::new(),
                commands,
                channel,
            },
        );
        Ok((
            RepoSessionPermit {
                id,
                runtime: self.clone(),
            },
            receiver,
        ))
    }

    fn update_binding(&self, id: u64, session: &WsSession) -> Result<(), RepoSessionRuntimeError> {
        let binding = match (
            session.active_branch.as_ref(),
            session.active_repo_id,
            session.catalog_membership_token(),
        ) {
            (None, Some(repo_id), Some(token)) if token.repo_id() == repo_id => {
                self.membership
                    .revalidate(token)
                    .map_err(|_| RepoSessionRuntimeError::MembershipRevoked)?;
                Some(RepoBoundSnapshot {
                    repo_id,
                    membership: token.clone(),
                    scope_nonce: session.scope_nonce(),
                })
            }
            _ => None,
        };
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        let registration = registry
            .sessions
            .get_mut(&id)
            .ok_or(RepoSessionRuntimeError::SessionMissing)?;
        registration.binding = binding;
        Ok(())
    }

    /// Invalidates every observer still bound to a RepoId after the durable
    /// membership cut. Each queued command carries the exact old token copied
    /// from that connection; no catalog permit is held while the registry is
    /// traversed or while commands are enqueued.
    pub(crate) fn invalidate_removed_repo_observers(
        &self,
        job_id: uuid::Uuid,
        removed_repo_id: RepoId,
        excluded_session_id: Option<u64>,
        final_list: FinalRepoListProjection,
    ) -> Result<usize, RepoSessionRuntimeError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        let mut invalidated = 0usize;
        for (session_id, registration) in &mut registry.sessions {
            if Some(*session_id) == excluded_session_id {
                continue;
            }
            let Some(binding) = registration.binding.as_ref() else {
                continue;
            };
            if binding.repo_id != removed_repo_id {
                continue;
            }
            let Some(next_scope_nonce) = binding.scope_nonce.checked_add(1) else {
                registration.channel.retire_session();
                registration.binding = None;
                continue;
            };
            let command = RepoSessionCommand::Removed {
                job_id,
                removed_repo_id,
                expected_membership: binding.membership.clone(),
                expected_scope_nonce: binding.scope_nonce,
                next_scope_nonce,
                final_list: final_list.clone(),
            };
            match registration.commands.try_send(command) {
                Ok(()) => invalidated += 1,
                Err(_) => registration.channel.retire_session(),
            }
            registration.binding = None;
        }
        Ok(invalidated)
    }

    pub(crate) fn register_lifecycle_observer(
        &self,
        session_id: u64,
        request_id: uuid::Uuid,
        expected_scope_nonce: u64,
        switch_nonce: u64,
    ) -> Result<(), RepoSessionRuntimeError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        let registration = registry
            .sessions
            .get_mut(&session_id)
            .ok_or(RepoSessionRuntimeError::SessionMissing)?;
        registration.lifecycle_observers.insert(
            request_id,
            RepoLifecycleSessionObserver {
                expected_scope_nonce,
                switch_nonce,
            },
        );
        Ok(())
    }

    pub(crate) fn clear_lifecycle_observer(
        &self,
        session_id: u64,
        request_id: uuid::Uuid,
    ) -> Result<(), RepoSessionRuntimeError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        let registration = registry
            .sessions
            .get_mut(&session_id)
            .ok_or(RepoSessionRuntimeError::SessionMissing)?;
        registration.lifecycle_observers.remove(&request_id);
        Ok(())
    }

    pub(crate) fn publish_lifecycle_settlement(
        &self,
        request_id: uuid::Uuid,
        job_id: uuid::Uuid,
        publication: super::repo_lifecycle_job_runtime::RepoLifecycleSettledPublication,
        final_list: FinalRepoListProjection,
    ) -> Result<Option<u64>, RepoSessionRuntimeError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| RepoSessionRuntimeError::Coordination)?;
        for (session_id, registration) in &mut registry.sessions {
            let Some(observer) = registration.lifecycle_observers.remove(&request_id) else {
                continue;
            };
            let command = RepoSessionCommand::LifecycleSettled {
                request_id,
                job_id,
                expected_scope_nonce: observer.expected_scope_nonce,
                switch_nonce: observer.switch_nonce,
                publication,
                final_list,
            };
            if registration.commands.try_send(command).is_err() {
                registration.channel.retire_session();
            }
            return Ok(Some(*session_id));
        }
        Ok(None)
    }

    #[cfg(test)]
    fn registered_sessions(&self) -> usize {
        self.registry
            .lock()
            .expect("repo session registry")
            .sessions
            .len()
    }
}

pub(crate) struct RepoSessionPermit {
    id: u64,
    runtime: Arc<RepoSessionRuntime>,
}

impl RepoSessionPermit {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn update(&self, session: &WsSession) -> Result<(), RepoSessionRuntimeError> {
        self.runtime.update_binding(self.id, session)
    }
}

impl Drop for RepoSessionPermit {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.runtime.registry.lock() {
            registry.sessions.remove(&self.id);
        }
    }
}

pub(crate) enum RepoSessionCommand {
    Removed {
        job_id: uuid::Uuid,
        removed_repo_id: RepoId,
        expected_membership: CatalogMembershipToken,
        expected_scope_nonce: u64,
        next_scope_nonce: u64,
        final_list: FinalRepoListProjection,
    },
    LifecycleSettled {
        request_id: uuid::Uuid,
        job_id: uuid::Uuid,
        expected_scope_nonce: u64,
        switch_nonce: u64,
        publication: super::repo_lifecycle_job_runtime::RepoLifecycleSettledPublication,
        final_list: FinalRepoListProjection,
    },
}

impl RepoSessionCommand {
    pub(crate) fn apply(
        self,
        state: &Arc<AppState>,
        session: &mut WsSession,
        channel: &DualChannel,
    ) -> bool {
        match self {
            Self::Removed {
                job_id,
                removed_repo_id,
                expected_membership,
                expected_scope_nonce,
                next_scope_nonce,
                final_list,
            } => {
                if session.active_repo_id != Some(removed_repo_id)
                    || session.catalog_membership_token() != Some(&expected_membership)
                    || session.scope_nonce() != expected_scope_nonce
                {
                    channel.retire_session();
                    return false;
                }
                state.revoke_source_control_write_grant_for_session(session);
                session.commit_no_scope(removed_repo_id, next_scope_nonce);
                channel.unicast(ServerMessage::RepoControl(
                    deve_core::protocol::RepoControlResponse::LocalRepoRemovalObserverInvalidated {
                        job_id,
                        removed_repo_id,
                        final_repo_list: final_list.entries,
                        scope: deve_core::protocol::RepoRemovalFinalScope::NoScope {
                            scope_nonce: deve_core::protocol::ScopeNonce::new(next_scope_nonce),
                        },
                    },
                ));
                true
            }
            Self::LifecycleSettled { .. } => {
                unreachable!("lifecycle settlement is applied by the WS transport adapter")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoSessionRuntimeError {
    Coordination,
    GenerationExhausted,
    SessionMissing,
    MembershipRevoked,
}

impl fmt::Display for RepoSessionRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordination => formatter.write_str("repo session registry is unavailable"),
            Self::GenerationExhausted => {
                formatter.write_str("repo session runtime identity exhausted")
            }
            Self::SessionMissing => formatter.write_str("repo session registration is missing"),
            Self::MembershipRevoked => {
                formatter.write_str("repo session catalog membership was revoked")
            }
        }
    }
}

impl std::error::Error for RepoSessionRuntimeError {}

#[cfg(test)]
mod tests;
