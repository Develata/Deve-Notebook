//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator

use super::*;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, Weak};

impl CatalogMembershipRuntime {
    pub(crate) fn for_ledger(ledger_dir: &Path) -> Result<Self, CatalogMembershipError> {
        let identity = std::fs::canonicalize(ledger_dir).map_err(|error| {
            CatalogMembershipError::InvalidLedgerIdentity(format!("{ledger_dir:?}: {error}"))
        })?;
        let registry = catalog_membership_registry();
        let mut registry = registry
            .lock()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        registry.retain(|_, runtime| runtime.strong_count() > 0);
        if let Some(runtime) = registry.get(&identity).and_then(Weak::upgrade) {
            return Ok(Self { inner: runtime });
        }
        let inner = Arc::new(CatalogMembershipInner {
            runtime_instance: Uuid::new_v4(),
            cut: Mutex::new(()),
            cut_authority: Mutex::new(None),
            state: RwLock::new(CatalogMembershipState::default()),
        });
        registry.insert(identity, Arc::downgrade(&inner));
        Ok(Self { inner })
    }
}

fn catalog_membership_registry() -> &'static Mutex<HashMap<PathBuf, Weak<CatalogMembershipInner>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Weak<CatalogMembershipInner>>>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}
