//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Repo and branch scope request/state contract for Web runtime clients.

use deve_core::models::RepoId;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoSwitchRequest {
    pub expected_name: String,
    pub repo_id: RepoId,
}

impl RepoSwitchRequest {
    pub fn exact(expected_name: String, repo_id: RepoId) -> Self {
        Self {
            expected_name,
            repo_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRenameRequest {
    pub repo_id: RepoId,
    pub current_name: String,
    pub new_name: String,
    pub expected_alias_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRemoveRequest {
    pub repo_id: RepoId,
    pub current_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingBranchTarget {
    Local,
    Shadow(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingBranchSwitch {
    pub target: PendingBranchTarget,
    pub switch_nonce: u64,
}

impl PendingBranchSwitch {
    pub fn new(target: PendingBranchTarget, switch_nonce: u64) -> Self {
        Self {
            target,
            switch_nonce,
        }
    }

    pub fn target(&self) -> &PendingBranchTarget {
        &self.target
    }

    pub fn into_target(self) -> PendingBranchTarget {
        self.target
    }
}

impl Deref for PendingBranchSwitch {
    type Target = PendingBranchTarget;

    fn deref(&self) -> &Self::Target {
        &self.target
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingRepoSwitchKind {
    Switch,
    RestoreSession,
    Create,
    RemoveCurrent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRepoSwitch {
    pub expected_name: String,
    pub expected_repo_id: Option<RepoId>,
    pub switch_nonce: u64,
    kind: PendingRepoSwitchKind,
}

impl PendingRepoSwitch {
    fn new(
        expected_name: impl Into<String>,
        expected_repo_id: Option<RepoId>,
        switch_nonce: u64,
        kind: PendingRepoSwitchKind,
    ) -> Self {
        Self {
            expected_name: expected_name.into(),
            expected_repo_id,
            switch_nonce,
            kind,
        }
    }

    pub fn switch(
        expected_name: impl Into<String>,
        expected_repo_id: RepoId,
        switch_nonce: u64,
    ) -> Self {
        Self::new(
            expected_name,
            Some(expected_repo_id),
            switch_nonce,
            PendingRepoSwitchKind::Switch,
        )
    }

    pub fn restore_session(
        expected_name: impl Into<String>,
        expected_repo_id: RepoId,
        switch_nonce: u64,
    ) -> Self {
        Self::new(
            expected_name,
            Some(expected_repo_id),
            switch_nonce,
            PendingRepoSwitchKind::RestoreSession,
        )
    }

    pub fn create(expected_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(
            expected_name,
            None,
            switch_nonce,
            PendingRepoSwitchKind::Create,
        )
    }

    #[allow(dead_code)] // R5 binds this only after explicit Execute admission.
    pub fn remove_current(current_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(
            current_name,
            None,
            switch_nonce,
            PendingRepoSwitchKind::RemoveCurrent,
        )
    }

    pub fn expected_name(&self) -> &str {
        &self.expected_name
    }

    pub fn restores_session_scope(&self) -> bool {
        self.kind == PendingRepoSwitchKind::RestoreSession
    }

    pub fn is_explicit_switch(&self) -> bool {
        self.kind == PendingRepoSwitchKind::Switch
    }

    pub fn bind_created_repo(&mut self, repo_id: RepoId) -> bool {
        if self.kind != PendingRepoSwitchKind::Create {
            return false;
        }
        match self.expected_repo_id {
            Some(expected) => expected == repo_id,
            None => {
                self.expected_repo_id = Some(repo_id);
                true
            }
        }
    }

    pub fn accepts_repo_switched(&self, repo_id: RepoId) -> bool {
        match self.kind {
            PendingRepoSwitchKind::Switch | PendingRepoSwitchKind::RestoreSession => {
                self.expected_repo_id == Some(repo_id)
            }
            PendingRepoSwitchKind::Create => self.expected_repo_id == Some(repo_id),
            PendingRepoSwitchKind::RemoveCurrent => true,
        }
    }
}

impl Deref for PendingRepoSwitch {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.expected_name
    }
}
