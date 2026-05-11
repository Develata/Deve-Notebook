//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::repo_catalog_runtime::RepoCatalogRuntime;
use crate::models::PeerId;
use anyhow::{Result, anyhow};

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in self.shadow_peer_dirs()? {
            let entries = self.scan_remote_repo_entries(&peer_id).map_err(|err| {
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
            if !self.list_remote_repo_names(&peer_id)?.is_empty() {
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
