//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract

use anyhow::{Result, anyhow};
use redb::Database;
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use crate::ledger::manager::local_repo_metadata_repair_support::ensure_local_repo_metadata_name_authorized;
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::ledger::source_control;

impl RepoManager {
    fn validate_local_repo_execution_identity(db: &Database, stem: &str) -> Result<RepoInfo> {
        let info = Self::read_local_repo_info_from_db(db)?.ok_or_else(|| {
            anyhow!(
                "Broken local repo {} while validating execution identity: repository metadata missing",
                stem
            )
        })?;
        let expected_stem = info.uuid.to_string();
        if expected_stem != stem {
            return Err(anyhow!(
                "Broken local repo {} while validating execution identity: physical RepoId does not match metadata RepoId {}",
                stem,
                info.uuid
            ));
        }
        Ok(info)
    }

    pub(crate) fn run_on_local_repo_stem<F, R>(&self, stem: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        if stem == self.local_repo_name {
            Self::validate_local_repo_execution_identity(self.local_db.as_ref(), stem)?;
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
                Self::validate_local_repo_execution_identity(db.as_ref(), stem)?;
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
        Self::validate_local_repo_execution_identity(db.as_ref(), stem)?;
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
        let mut display_matches = Vec::new();
        if selector == self.local_repo_name {
            let info = Self::validate_local_repo_execution_identity(
                self.local_db.as_ref(),
                &self.local_repo_name,
            )?;
            if self.is_local_repo_removed(info.uuid)? {
                return Ok(None);
            }
            return Ok(Some(self.local_repo_name.clone()));
        }
        if let Some(info) = Self::read_local_repo_info_from_db(&self.local_db)?
            && !self.is_local_repo_removed(info.uuid)?
        {
            ensure_local_repo_metadata_name_authorized(
                &self.ledger_dir,
                &self.local_repo_name,
                &info,
            )?;
            if info.name == selector {
                display_matches.push(self.local_repo_name.clone());
            }
        }
        let local_dir = Self::checked_local_dir_for(&self.ledger_dir, "resolving local selector")?;
        for (path, stem) in redb_repo_entries(&local_dir, "resolving local selector")? {
            if stem == self.local_repo_name {
                continue;
            }
            let info = Self::read_required_local_repo_info_from_path(
                &path,
                &stem,
                "resolving local selector",
            )
            .map_err(|err| {
                anyhow!(
                    "Broken local repo {} while resolving selector {}: {}",
                    stem,
                    selector,
                    err
                )
            })?;
            if self.is_local_repo_removed(info.uuid)? {
                continue;
            }
            if stem == selector {
                if info.uuid.to_string() != stem {
                    return Err(anyhow!(
                        "Broken local repo {} while resolving selector {}: physical RepoId does not match metadata RepoId {}",
                        stem,
                        selector,
                        info.uuid
                    ));
                }
                return Ok(Some(stem));
            }
            ensure_local_repo_metadata_name_authorized(&self.ledger_dir, &stem, &info)?;
            if info.name == selector {
                display_matches.push(stem);
            }
        }
        match display_matches.len() {
            0 => Ok(None),
            1 => Ok(display_matches.into_iter().next()),
            _ => {
                display_matches.sort();
                Err(ambiguous_local_selector(selector, &display_matches))
            }
        }
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

fn ambiguous_local_selector(selector: &str, matches: &[String]) -> anyhow::Error {
    anyhow!(
        "Ambiguous local repository selector {} matched {:?}",
        selector,
        matches
    )
}
