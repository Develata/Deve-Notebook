//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Bounded thin-client state for the two-frame remove-scope partial outcome.

use crate::runtime::domain::PendingRepoSwitch;
use deve_core::models::RepoId;
use deve_core::protocol::{RepoListEntry, ServerErrorCode};

pub const REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS: u64 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoveScopePartialKind {
    Initiator { switch_nonce: u64 },
    Observer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoveScopePartialStageKey {
    pub connection_epoch: u64,
    pub kind: RemoveScopePartialKind,
    pub scope_nonce: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveScopePartialStage {
    pub connection_epoch: u64,
    pub kind: RemoveScopePartialKind,
    pub scope_nonce: u64,
    pub removed_repo_id: RepoId,
    pub repos: Vec<String>,
    pub repo_entries: Vec<RepoListEntry>,
    deadline_mono_ms: u64,
}

impl RemoveScopePartialStage {
    pub fn new(
        connection_epoch: u64,
        kind: RemoveScopePartialKind,
        scope_nonce: u64,
        removed_repo_id: RepoId,
        repos: Vec<String>,
        repo_entries: Vec<RepoListEntry>,
        now_mono_ms: u64,
    ) -> Self {
        Self {
            connection_epoch,
            kind,
            scope_nonce,
            removed_repo_id,
            repos,
            repo_entries,
            deadline_mono_ms: now_mono_ms.saturating_add(REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS),
        }
    }

    pub fn key(&self) -> RemoveScopePartialStageKey {
        RemoveScopePartialStageKey {
            connection_epoch: self.connection_epoch,
            kind: self.kind,
            scope_nonce: self.scope_nonce,
        }
    }

    pub fn is_expired_at(&self, now_mono_ms: u64) -> bool {
        now_mono_ms >= self.deadline_mono_ms
    }
}

pub struct RepoListStageInput<'a> {
    pub request_id: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub scope_nonce: Option<u64>,
    pub current_scope_nonce: u64,
    pub current_repo_id: RepoId,
    pub pending_branch_switch: bool,
    pub pending_repo_switch: Option<&'a PendingRepoSwitch>,
    pub repo_entries: &'a [RepoListEntry],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoListStageDecision {
    NotPartial,
    Stage(RemoveScopePartialKind),
    Retire,
}

pub fn classify_repo_list(
    existing: Option<&RemoveScopePartialStage>,
    input: RepoListStageInput<'_>,
) -> RepoListStageDecision {
    if existing.is_some() {
        return RepoListStageDecision::Retire;
    }
    if input.request_id.is_some()
        || input.branch.is_some()
        || input.pending_branch_switch
        || input.repo_entries.is_empty()
    {
        return RepoListStageDecision::NotPartial;
    }
    let Some(scope_nonce) = input.scope_nonce else {
        return RepoListStageDecision::NotPartial;
    };
    if scope_nonce <= input.current_scope_nonce
        || input
            .repo_entries
            .iter()
            .any(|entry| entry.repo_id == input.current_repo_id)
    {
        return RepoListStageDecision::NotPartial;
    }

    match input.pending_repo_switch {
        Some(pending) if pending.is_remove_current() && pending.switch_nonce == scope_nonce => {
            RepoListStageDecision::Stage(RemoveScopePartialKind::Initiator {
                switch_nonce: pending.switch_nonce,
            })
        }
        Some(_) => RepoListStageDecision::NotPartial,
        None => RepoListStageDecision::Stage(RemoveScopePartialKind::Observer),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolStageDecision {
    NotPartial,
    Commit,
    Retire,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoSwitchedStageDecision {
    NotPartial,
    Commit,
    Retire,
}

pub fn classify_repo_switched(
    stage: Option<&RemoveScopePartialStage>,
    current_connection_epoch: u64,
    repo_id: RepoId,
    name: &str,
    switch_nonce: Option<u64>,
    now_mono_ms: u64,
) -> RepoSwitchedStageDecision {
    let Some(stage) = stage else {
        return RepoSwitchedStageDecision::NotPartial;
    };
    if stage.connection_epoch != current_connection_epoch || stage.is_expired_at(now_mono_ms) {
        return RepoSwitchedStageDecision::Retire;
    }
    let RemoveScopePartialKind::Initiator {
        switch_nonce: expected,
    } = stage.kind
    else {
        return RepoSwitchedStageDecision::Retire;
    };
    let exact_fallback = repo_id != stage.removed_repo_id
        && stage
            .repo_entries
            .iter()
            .any(|entry| entry.repo_id == repo_id && entry.display_alias == name);
    if switch_nonce == Some(expected) && switch_nonce == Some(stage.scope_nonce) && exact_fallback {
        RepoSwitchedStageDecision::Commit
    } else {
        RepoSwitchedStageDecision::Retire
    }
}

pub fn classify_protocol_error(
    stage: Option<&RemoveScopePartialStage>,
    current_connection_epoch: u64,
    code: ServerErrorCode,
    switch_nonce: Option<u64>,
    scope_nonce: Option<u64>,
    now_mono_ms: u64,
) -> ProtocolStageDecision {
    let Some(stage) = stage else {
        return ProtocolStageDecision::NotPartial;
    };
    if stage.connection_epoch != current_connection_epoch || stage.is_expired_at(now_mono_ms) {
        return ProtocolStageDecision::Retire;
    }
    let switch_matches = match stage.kind {
        RemoveScopePartialKind::Initiator {
            switch_nonce: expected,
        } => switch_nonce == Some(expected),
        RemoveScopePartialKind::Observer => switch_nonce.is_none(),
    };
    if code == ServerErrorCode::ScRepoNotSelected
        && scope_nonce == Some(stage.scope_nonce)
        && switch_matches
    {
        ProtocolStageDecision::Commit
    } else {
        ProtocolStageDecision::Retire
    }
}

#[cfg(target_arch = "wasm32")]
pub fn monotonic_now_ms() -> u64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now().max(0.0) as u64)
        .unwrap_or(0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn monotonic_now_ms() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();
    START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
