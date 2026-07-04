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
    pub selector_name: String,
    pub expected_name: String,
    pub repo_id: Option<RepoId>,
}

impl RepoSwitchRequest {
    pub fn by_name(name: String) -> Self {
        Self {
            selector_name: name.clone(),
            expected_name: name,
            repo_id: None,
        }
    }

    pub fn exact(selector_name: String, expected_name: String, repo_id: RepoId) -> Self {
        Self {
            selector_name,
            expected_name,
            repo_id: Some(repo_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRenameRequest {
    pub repo_id: RepoId,
    pub current_name: String,
    pub new_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRemoveRequest {
    pub repo_id: RepoId,
    pub current_name: String,
    pub fallback_name: Option<String>,
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
    Create,
    RenameCurrent,
    RemoveCurrent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRepoSwitch {
    pub expected_name: String,
    pub switch_nonce: u64,
    kind: PendingRepoSwitchKind,
}

impl PendingRepoSwitch {
    fn new(
        expected_name: impl Into<String>,
        switch_nonce: u64,
        kind: PendingRepoSwitchKind,
    ) -> Self {
        Self {
            expected_name: expected_name.into(),
            switch_nonce,
            kind,
        }
    }

    pub fn switch(expected_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(expected_name, switch_nonce, PendingRepoSwitchKind::Switch)
    }

    pub fn create(expected_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(expected_name, switch_nonce, PendingRepoSwitchKind::Create)
    }

    pub fn rename_current(expected_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(
            expected_name,
            switch_nonce,
            PendingRepoSwitchKind::RenameCurrent,
        )
    }

    pub fn remove_current(expected_name: impl Into<String>, switch_nonce: u64) -> Self {
        Self::new(
            expected_name,
            switch_nonce,
            PendingRepoSwitchKind::RemoveCurrent,
        )
    }

    pub fn expected_name(&self) -> &str {
        &self.expected_name
    }
}

impl Deref for PendingRepoSwitch {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.expected_name
    }
}
