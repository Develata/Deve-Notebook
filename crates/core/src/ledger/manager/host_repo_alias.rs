//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 14_commands#repo-alias-command-contract
//!
//! Host-local display aliases. This runtime owns the only mutable alias store;
//! aliases never enter Ledger, locator, workspace, sync, or remote-import state.

mod import;
mod membership;
mod model;
mod store;

pub use model::{
    HostRepoAliasBinding, HostRepoAliasCleanupDisposition, HostRepoAliasError,
    HostRepoAliasImportSummary, HostRepoAliasImportWarning, HostRepoAliasImportWarningReason,
    HostRepoAliasRemovalPlan, HostRepoAliasSetResult, HostRepoAliasValidationError,
};

use self::import::{ParsedImport, evaluate_import};
use self::membership::{LocalRepoAdmission, LocalRepoMembershipSnapshot, checked_local_dir};
use self::store::{AliasStore, AliasStoreGuard};
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use std::path::{Path, PathBuf};

pub const HOST_REPO_ALIAS_IMPORT_MAX_BYTES: usize = 1024 * 1024;
/// Host-only alias capability opened without creating or repairing any repo.
pub struct HostRepoAliasRuntime {
    ledger_dir: PathBuf,
}

impl HostRepoAliasRuntime {
    /// Normalize one display alias without opening or mutating host state.
    ///
    /// Lifecycle admission uses this typed boundary so alias validation remains
    /// owned by the alias runtime instead of being copied into transport code.
    pub fn normalize_alias(alias: &str) -> Result<String, HostRepoAliasValidationError> {
        model::normalize_alias(alias)
    }

    pub fn open_existing(ledger_dir: impl AsRef<Path>) -> Result<Self, HostRepoAliasError> {
        let ledger_dir = ledger_dir.as_ref().to_path_buf();
        checked_local_dir(&ledger_dir, "opening host repo alias runtime")?;
        Ok(Self { ledger_dir })
    }

    fn for_manager(manager: &RepoManager) -> Self {
        Self {
            ledger_dir: manager.ledger_dir.clone(),
        }
    }

    pub fn binding(&self, repo_id: RepoId) -> Result<HostRepoAliasBinding, HostRepoAliasError> {
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        self.require_active_local_repo(&membership, repo_id)?;
        let store = AliasStore::load(&self.ledger_dir)?;
        Ok(store.binding_or_fallback(repo_id))
    }

    pub fn set_alias(
        &self,
        repo_id: RepoId,
        alias: &str,
        expected_alias_revision: u64,
    ) -> Result<HostRepoAliasSetResult, HostRepoAliasError> {
        let alias = model::normalize_alias(alias)?;
        let _store_guard = AliasStoreGuard::acquire(&self.ledger_dir)?;
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        self.require_active_local_repo(&membership, repo_id)?;
        let mut store = AliasStore::load(&self.ledger_dir)?;
        let result = store.set(repo_id, alias, expected_alias_revision)?;
        if result.changed {
            store.publish(&self.ledger_dir)?;
        }
        Ok(result)
    }

    pub fn export_json(&self) -> Result<String, HostRepoAliasError> {
        let store = AliasStore::load(&self.ledger_dir)?;
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        let mut aliases = Vec::new();
        for binding in store.bindings() {
            match membership.admit(binding.repo_id)? {
                LocalRepoAdmission::Active => aliases.push(model::HostRepoAliasExportEntry {
                    repo_id: binding.repo_id,
                    alias: binding.alias.clone(),
                }),
                LocalRepoAdmission::Unknown => {}
            }
        }
        aliases.sort_by_key(|entry| entry.repo_id);
        let document = model::HostRepoAliasExportDocument {
            format: model::EXPORT_FORMAT,
            version: model::EXPORT_VERSION,
            aliases,
        };
        let mut json = serde_json::to_string_pretty(&document)?;
        json.push('\n');
        Ok(json)
    }

    pub fn preview_import_json(
        &self,
        input: &[u8],
    ) -> Result<HostRepoAliasImportSummary, HostRepoAliasError> {
        let parsed = ParsedImport::parse(input)?;
        let store = AliasStore::load(&self.ledger_dir)?;
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        evaluate_import(&parsed, &store, |repo_id| membership.admit(repo_id))
            .map(|evaluation| evaluation.summary)
    }

    pub fn apply_import_json(
        &self,
        input: &[u8],
    ) -> Result<HostRepoAliasImportSummary, HostRepoAliasError> {
        // Parsing occurs before the lock, but admission and store state are
        // always recomputed after the exclusive cross-process lock is held.
        let parsed = ParsedImport::parse(input)?;
        let _store_guard = AliasStoreGuard::acquire(&self.ledger_dir)?;
        let mut store = AliasStore::load(&self.ledger_dir)?;
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        let evaluation = evaluate_import(&parsed, &store, |repo_id| membership.admit(repo_id))?;
        let changed = evaluation.apply(&mut store)?;
        if changed {
            store.publish(&self.ledger_dir)?;
        }
        Ok(evaluation.summary)
    }

    pub fn prepare_removal(
        &self,
        repo_id: RepoId,
    ) -> Result<HostRepoAliasRemovalPlan, HostRepoAliasError> {
        let membership = LocalRepoMembershipSnapshot::load(&self.ledger_dir)?;
        self.require_active_local_repo(&membership, repo_id)?;
        let binding = AliasStore::load(&self.ledger_dir)?.binding_or_fallback(repo_id);
        Ok(HostRepoAliasRemovalPlan { binding })
    }

    pub fn cleanup_removal(
        &self,
        plan: &HostRepoAliasRemovalPlan,
    ) -> Result<HostRepoAliasCleanupDisposition, HostRepoAliasError> {
        let _store_guard = AliasStoreGuard::acquire(&self.ledger_dir)?;
        membership::require_removed(&self.ledger_dir, plan.repo_id())?;
        let mut store = AliasStore::load(&self.ledger_dir)?;
        let current = store.binding_or_fallback(plan.repo_id());
        if current != plan.binding {
            return Err(HostRepoAliasError::RevisionConflict {
                repo_id: plan.repo_id(),
                expected: plan.binding.alias_revision,
                current: current.alias_revision,
            });
        }
        if plan.binding.alias_revision == 0 {
            return Ok(HostRepoAliasCleanupDisposition::AlreadyAbsent);
        }
        if !store.remove_exact(&plan.binding) {
            return Err(HostRepoAliasError::StoreInvalid(
                "exact alias row disappeared during cleanup".to_string(),
            ));
        }
        store.publish(&self.ledger_dir)?;
        Ok(HostRepoAliasCleanupDisposition::Deleted)
    }

    pub fn removal_is_absent(
        &self,
        plan: &HostRepoAliasRemovalPlan,
    ) -> Result<bool, HostRepoAliasError> {
        let current = AliasStore::load(&self.ledger_dir)?.binding_or_fallback(plan.repo_id());
        Ok(current.alias_revision == 0)
    }

    fn require_active_local_repo(
        &self,
        membership: &LocalRepoMembershipSnapshot,
        repo_id: RepoId,
    ) -> Result<(), HostRepoAliasError> {
        match membership.admit(repo_id)? {
            LocalRepoAdmission::Active => Ok(()),
            LocalRepoAdmission::Unknown => Err(HostRepoAliasError::UnknownLocalRepo(repo_id)),
        }
    }
}

impl RepoManager {
    pub fn host_repo_alias_runtime(&self) -> HostRepoAliasRuntime {
        HostRepoAliasRuntime::for_manager(self)
    }
}

#[cfg(test)]
mod tests;
