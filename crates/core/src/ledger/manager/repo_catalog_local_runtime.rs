//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/projection#projection-locator-contract

use crate::ledger::manager::repo_catalog_runtime::RepoCatalogRuntime;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::{Result, anyhow};

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn list_local_display_names(&self) -> Result<Vec<String>> {
        self.refresh_local_catalog()?;
        let mut named = Vec::new();
        for repo_id in self.manager.normal_repo_catalog_ids()? {
            let stem = repo_id.to_string();
            let info = self
                .manager
                .run_on_local_repo_stem(&stem, |db| {
                    let info = RepoManager::read_local_repo_info_from_db(db)?.ok_or_else(|| {
                        anyhow!(
                            "Broken local repo {} while listing repos: repository metadata missing",
                            stem
                        )
                    })?;
                    Ok(info)
                })
                .map_err(|err| {
                    anyhow!("Broken local repo {} while listing repos: {}", stem, err)
                })?;
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
        Ok(self
            .local_repo_info_snapshot("listing execution names")?
            .into_iter()
            .map(|(stem, _)| stem)
            .collect())
    }

    fn local_repo_info_snapshot(&self, context: &str) -> Result<Vec<(String, RepoInfo)>> {
        let mut repos = Vec::new();
        for repo_id in self.manager.normal_repo_catalog_ids()? {
            let stem = repo_id.to_string();
            let info = self
                .manager
                .run_on_local_repo_stem(&stem, |db| {
                    RepoManager::read_local_repo_info_from_db(db)?.ok_or_else(|| {
                        anyhow!(
                            "Broken local repo {} while {}: repository metadata missing",
                            stem,
                            context
                        )
                    })
                })
                .map_err(|err| anyhow!("Broken local repo {} while {}: {}", stem, context, err))?;
            repos.push((stem, info));
        }
        repos.sort_by(|(left, _), (right, _)| left.cmp(right));
        Ok(repos)
    }
}
