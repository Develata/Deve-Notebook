//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract

use crate::ledger::manager::local_repo_metadata_repair_support::{
    ensure_cataloged_repo_name_canonical, ensure_local_repo_metadata_identity,
};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::ledger::source_control;
use anyhow::{Result, anyhow};
use redb::Database;

impl RepoManager {
    pub(crate) fn validate_local_repo_execution_identity(
        db: &Database,
        stem: &str,
    ) -> Result<RepoInfo> {
        let info = Self::read_local_repo_info_from_db(db)?.ok_or_else(|| {
            anyhow!(
                "Broken local repo {} while validating execution identity: repository metadata missing",
                stem
            )
        })?;
        ensure_local_repo_metadata_identity(stem, &info)?;
        ensure_cataloged_repo_name_canonical(stem, &info)?;
        Ok(info)
    }

    pub(crate) fn run_on_local_repo_stem<F, R>(&self, stem: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        let lease = self
            .lease_local_authority_stem(stem)
            .map_err(|err| anyhow!("Broken local repo {} while opening database: {}", stem, err))?;
        Self::validate_local_repo_execution_identity(lease.db(), stem)?;
        source_control::validate_tables(lease.db()).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                stem,
                err
            )
        })?;
        f(lease.db())
    }
    pub(crate) fn resolve_local_repo_stem(&self, selector: &str) -> Result<Option<String>> {
        let normal_repo_ids = self.normal_repo_catalog_ids()?;
        if let Ok(repo_id) = uuid::Uuid::parse_str(selector)
            && normal_repo_ids.contains(&repo_id)
        {
            let stem = repo_id.to_string();
            self.run_on_local_repo_stem(&stem, |_| Ok(()))
                .map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while resolving exact UUID {}: {}",
                        stem,
                        selector,
                        err
                    )
                })?;
            return Ok(Some(stem));
        }

        let mut display_matches = Vec::new();
        for repo_id in normal_repo_ids {
            let stem = repo_id.to_string();
            let info = self
                .run_on_local_repo_stem(&stem, |db| {
                    Self::read_local_repo_info_from_db(db)?.ok_or_else(|| {
                        anyhow!(
                            "Broken local repo {} while resolving local selector: repository metadata missing",
                            stem
                        )
                    })
                })
            .map_err(|err| {
                anyhow!(
                    "Broken local repo {} while resolving selector {}: {}",
                    stem,
                    selector,
                    err
                )
            })?;
            ensure_local_repo_metadata_identity(&stem, &info)?;
            ensure_cataloged_repo_name_canonical(&stem, &info)?;
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
}

fn ambiguous_local_selector(selector: &str, matches: &[String]) -> anyhow::Error {
    anyhow!(
        "Ambiguous local repository selector {} matched {:?}",
        selector,
        matches
    )
}
