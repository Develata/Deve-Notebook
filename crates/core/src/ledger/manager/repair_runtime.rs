//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract
//!   - 04_repository#repo-catalog-contract
//!   - 03_storage/index#repo-runtime-layout

use crate::ledger::manager::local_repo_metadata_repair::repair_local_repo_metadata;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::{Context, Result};

pub(crate) struct RepairRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> RepairRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn repair_local_repo_catalog(&self) -> Result<()> {
        repair_local_repo_metadata(
            &self.manager.ledger_dir,
            &self.manager.local_repo_name,
            self.manager.local_db.as_ref(),
        )?;
        RepoManager::repair_local_repo_source_control_tables(
            &self.manager.ledger_dir,
            &self.manager.local_repo_name,
            self.manager.local_db.as_ref(),
        )
    }

    pub(crate) fn repair_remote_repo_catalogs(&self) -> Result<()> {
        let remotes_dir = self.manager.checked_remotes_dir()?;
        let mut peers = Vec::new();
        for entry in std::fs::read_dir(remotes_dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                return Err(anyhow::anyhow!(
                    "Broken shadow peer entry {:?} while repairing catalogs: invalid directory name",
                    path
                ));
            };
            if name == ".invalid" {
                continue;
            }
            if name.starts_with('.') || name.is_empty() {
                return Err(anyhow::anyhow!(
                    "Broken shadow peer entry {:?} while repairing catalogs: unexpected hidden directory",
                    path
                ));
            }
            if !entry.file_type()?.is_dir() {
                return Err(anyhow::anyhow!(
                    "Broken shadow peer entry {:?} while repairing catalogs: expected directory",
                    path
                ));
            }
            peers.push(PeerId::new(name));
        }
        peers.sort_by_key(|peer_id| peer_id.to_string());
        for peer_id in peers {
            self.manager
                .repo_catalog_runtime()
                .repair_remote_repo_catalog(&peer_id)
                .with_context(|| {
                    format!("Broken shadow peer {} while repairing catalogs", peer_id)
                })?;
            self.manager
                .repo_catalog_runtime()
                .scan_remote_repo_entries(&peer_id)
                .with_context(|| {
                    format!("Broken shadow peer {} while repairing catalogs", peer_id)
                })?;
        }
        Ok(())
    }
}

impl RepoManager {
    pub fn repair_local_repo_catalog(&self) -> Result<()> {
        self.repair_runtime().repair_local_repo_catalog()
    }

    pub fn repair_remote_repo_catalogs(&self) -> Result<()> {
        self.repair_runtime().repair_remote_repo_catalogs()
    }

    pub(crate) fn repair_runtime(&self) -> RepairRuntime<'_> {
        RepairRuntime::new(self)
    }
}
