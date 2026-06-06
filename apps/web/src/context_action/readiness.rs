//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! Context action readiness snapshots owned by Web flow coordination.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextActionScope {
    pub repo_id: Option<String>,
    pub scope_nonce: u64,
}

impl ContextActionScope {
    pub fn new(repo_id: Option<String>, scope_nonce: u64) -> Self {
        Self {
            repo_id,
            scope_nonce,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextActionReadiness {
    pub scope: ContextActionScope,
    pub readonly: bool,
    pub write_blocked: bool,
}

impl ContextActionReadiness {
    pub fn new(scope: ContextActionScope, readonly: bool, write_blocked: bool) -> Self {
        Self {
            scope,
            readonly,
            write_blocked,
        }
    }

    #[cfg(test)]
    pub fn from_readonly(readonly: bool) -> Self {
        Self::new(ContextActionScope::default(), readonly, false)
    }

    #[cfg(test)]
    pub fn with_scope(mut self, scope: ContextActionScope) -> Self {
        self.scope = scope;
        self
    }

    #[cfg(test)]
    pub fn with_write_blocked(mut self, write_blocked: bool) -> Self {
        self.write_blocked = write_blocked;
        self
    }
}
