//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::repo_catalog_runtime::RepoCatalogRuntime;
use crate::ledger::manager::types::RepoManager;
use anyhow::{Result, anyhow};

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn list_local_display_names(&self) -> Result<Vec<String>> {
        self.refresh_local_catalog()?;
        let target_dir =
            RepoManager::checked_local_dir_for(&self.manager.ledger_dir, "listing repos")?;

        let mut named = Vec::new();
        for (path, stem) in redb_repo_entries(&target_dir, "listing repos")? {
            let info = if stem == self.manager.local_repo_name {
                self.manager.get_repo_info()?.ok_or_else(|| {
                    anyhow!(
                        "Broken local repo {} while listing repos: repository metadata missing",
                        stem
                    )
                })?
            } else {
                RepoManager::read_required_repo_info_from_path(&path, &stem, "listing repos")
                    .map_err(|err| {
                        anyhow!("Broken local repo {} while listing repos: {}", stem, err)
                    })?
            };
            if self.manager.is_local_repo_removed(info.uuid)? {
                continue;
            }
            named.push((stem, info.name));
        }

        let mut counts = std::collections::HashMap::<String, usize>::new();
        for (_, display) in &named {
            *counts.entry(display.clone()).or_default() += 1;
        }
        let mut repos = named
            .into_iter()
            .map(|(stem, display)| {
                if counts.get(&display).copied().unwrap_or(0) > 1 {
                    stem
                } else {
                    display
                }
            })
            .collect::<Vec<_>>();

        repos.sort();
        Ok(repos)
    }

    pub(crate) fn list_local_execution_names(&self) -> Result<Vec<String>> {
        self.refresh_local_catalog()?;
        let local_dir = RepoManager::checked_local_dir_for(
            &self.manager.ledger_dir,
            "listing execution names",
        )?;

        let mut repos = Vec::new();
        for (path, stem) in redb_repo_entries(&local_dir, "listing execution names")? {
            if stem == self.manager.local_repo_name {
                let info = self.manager.get_repo_info()?.ok_or_else(|| {
                    anyhow!(
                        "Broken local repo {} while listing execution names: repository metadata missing",
                        stem
                    )
                })?;
                if self.manager.is_local_repo_removed(info.uuid)? {
                    continue;
                }
                repos.push(stem);
                continue;
            }
            let info = RepoManager::read_required_repo_info_from_path(
                &path,
                &stem,
                "listing execution names",
            )
            .map_err(|err| {
                anyhow!(
                    "Broken local repo {} while listing execution names: {}",
                    stem,
                    err
                )
            })?;
            if self.manager.is_local_repo_removed(info.uuid)? {
                continue;
            }
            repos.push(stem);
        }
        repos.sort();
        Ok(repos)
    }
}
