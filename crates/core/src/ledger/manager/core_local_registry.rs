//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract

use anyhow::{Result, anyhow};
use redb::Database;
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;

impl RepoManager {
    pub(crate) fn run_on_local_repo_stem<F, R>(&self, stem: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        if stem == self.local_repo_name {
            source_control::validate_tables(self.local_db.as_ref()).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating source control tables: {}",
                    self.local_repo_name,
                    err
                )
            })?;
            return f(self.local_db.as_ref());
        }
        {
            let guard = self.read_extra_local_dbs()?;
            if let Some(db) = guard.get(stem) {
                source_control::validate_tables(db.as_ref()).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while validating source control tables: {}",
                        stem,
                        err
                    )
                })?;
                return f(db);
            }
        }
        let db = self
            .get_or_open_local_db(stem)
            .map_err(|err| anyhow!("Broken local repo {} while opening database: {}", stem, err))?;
        source_control::validate_tables(db.as_ref()).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                stem,
                err
            )
        })?;
        {
            let mut guard = self.write_extra_local_dbs()?;
            if let std::collections::hash_map::Entry::Vacant(e) = guard.entry(stem.to_string()) {
                e.insert(db);
            }
        }
        let guard = self.read_extra_local_dbs()?;
        guard
            .get(stem)
            .map(|db| f(db.as_ref()))
            .transpose()?
            .ok_or_else(|| anyhow!("Failed into cache repo"))
    }
    pub(crate) fn resolve_local_repo_stem(&self, selector: &str) -> Result<Option<String>> {
        if selector == self.local_repo_name {
            return Ok(Some(self.local_repo_name.clone()));
        }
        if let Some(info) = Self::read_repo_info_from_db(&self.local_db)?
            && info.name == selector
        {
            return Err(local_selector_metadata_drift(
                &self.local_repo_name,
                selector,
                &info.name,
            ));
        }
        let local_dir = Self::checked_local_dir_for(&self.ledger_dir, "resolving local selector")?;
        for (path, stem) in redb_repo_entries(&local_dir, "resolving local selector")? {
            if stem == self.local_repo_name {
                continue;
            }
            let info =
                Self::read_required_repo_info_from_path(&path, &stem, "resolving local selector")
                    .map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while resolving selector {}: {}",
                        stem,
                        selector,
                        err
                    )
                })?;
            if stem != selector && info.name == selector {
                return Err(local_selector_metadata_drift(&stem, selector, &info.name));
            }
            if stem == selector {
                return Ok(Some(stem));
            }
        }
        Ok(None)
    }

    fn read_extra_local_dbs(&self) -> Result<RwLockReadGuard<'_, HashMap<String, Arc<Database>>>> {
        self.extra_local_dbs
            .read()
            .map_err(|_| anyhow!("Local repo registry lock poisoned"))
    }

    fn write_extra_local_dbs(
        &self,
    ) -> Result<RwLockWriteGuard<'_, HashMap<String, Arc<Database>>>> {
        self.extra_local_dbs
            .write()
            .map_err(|_| anyhow!("Local repo registry lock poisoned"))
    }
}

fn local_selector_metadata_drift(stem: &str, selector: &str, name: &str) -> anyhow::Error {
    anyhow!(
        "Broken local repo {} while resolving selector {}: metadata name drifted to {}",
        stem,
        selector,
        name
    )
}
