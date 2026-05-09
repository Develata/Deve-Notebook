//! plan_ref:
//!   - 06_repository#repo-catalog-repair-contract
//!   - 06_repository#repo-catalog-contract
//!   - 04_storage#repo-runtime-layout

use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::{Context, Result};

impl RepoManager {
    pub fn repair_local_repo_catalog(&self) -> Result<()> {
        Self::repair_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
            self.vault_root.as_deref(),
            true,
        )?;
        Self::repair_local_repo_source_control_tables(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )
    }

    pub fn repair_remote_repo_catalogs(&self) -> Result<()> {
        let remotes_dir = self.checked_remotes_dir()?;
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
            self.repair_remote_repo_catalog(&peer_id).with_context(|| {
                format!("Broken shadow peer {} while repairing catalogs", peer_id)
            })?;
            self.scan_remote_repo_entries(&peer_id).with_context(|| {
                format!("Broken shadow peer {} while repairing catalogs", peer_id)
            })?;
        }
        Ok(())
    }
}
