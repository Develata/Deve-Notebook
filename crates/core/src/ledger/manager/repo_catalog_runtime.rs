//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::{Result, anyhow};

impl RepoManager {
    pub(crate) fn refresh_local_repo_catalog(&self) -> Result<()> {
        Self::validate_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )?;
        Self::validate_local_repo_source_control_tables(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )
    }
}

pub(crate) struct RepoCatalogRuntime<'a> {
    pub(super) manager: &'a RepoManager,
}

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn refresh_local_catalog(&self) -> Result<()> {
        self.manager.refresh_local_repo_catalog()
    }

    pub(crate) fn list_local_display_names(&self) -> Result<Vec<String>> {
        self.list_repos(None)
    }

    pub(crate) fn list_local_execution_names(&self) -> Result<Vec<String>> {
        self.manager.list_local_repo_names_for_execution()
    }

    pub(crate) fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>> {
        if let Some(peer_id) = peer_id {
            return self.manager.list_remote_repo_names(peer_id);
        }
        self.refresh_local_catalog()?;
        let target_dir =
            RepoManager::checked_local_dir_for(&self.manager.ledger_dir, "listing repos")?;

        let mut named = Vec::new();
        for (path, stem) in redb_repo_entries(&target_dir, "listing repos")? {
            let display = if stem == self.manager.local_repo_name {
                self.manager
                    .get_repo_info()?
                    .ok_or_else(|| {
                        anyhow!(
                            "Broken local repo {} while listing repos: repository metadata missing",
                            stem
                        )
                    })?
                    .name
            } else {
                RepoManager::read_required_repo_info_from_path(&path, &stem, "listing repos")
                    .map_err(|err| {
                        anyhow!("Broken local repo {} while listing repos: {}", stem, err)
                    })?
                    .name
            };
            named.push((stem, display));
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

    pub(crate) fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in self.shadow_peer_dirs()? {
            let entries = self
                .manager
                .scan_remote_repo_entries(&peer_id)
                .map_err(|err| {
                    anyhow!(
                        "Broken shadow peer {} while listing shadows: {}",
                        peer_id,
                        err
                    )
                })?;
            if !entries.is_empty() {
                peers.push(peer_id);
            }
        }
        peers.sort();
        Ok(peers)
    }

    pub(crate) fn list_switchable_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in self.shadow_peer_dirs()? {
            if !self.manager.list_remote_repo_names(&peer_id)?.is_empty() {
                peers.push(peer_id);
            }
        }
        peers.sort();
        Ok(peers)
    }

    fn shadow_peer_dirs(&self) -> Result<Vec<PeerId>> {
        let remotes_dir = self.manager.checked_remotes_dir()?;
        let mut peers = Vec::new();
        for entry in std::fs::read_dir(&remotes_dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                return Err(anyhow!(
                    "Broken shadow peer entry {:?} while listing shadows: invalid directory name",
                    path
                ));
            };
            if name == ".invalid" {
                continue;
            }
            if name.starts_with('.') || name.is_empty() {
                return Err(anyhow!(
                    "Broken shadow peer entry {:?} while listing shadows: unexpected hidden directory",
                    path
                ));
            }
            if !entry.file_type()?.is_dir() {
                return Err(anyhow!(
                    "Broken shadow peer entry {:?} while listing shadows: expected directory",
                    path
                ));
            }
            peers.push(PeerId::new(name));
        }
        peers.sort();
        Ok(peers)
    }
}

impl RepoManager {
    pub(crate) fn repo_catalog_runtime(&self) -> RepoCatalogRuntime<'_> {
        RepoCatalogRuntime::new(self)
    }
}
