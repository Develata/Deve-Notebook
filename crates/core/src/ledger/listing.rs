// crates\core\src\ledger
//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!   - 06_repository#tree-projection-contract
//!   - 06_repository#repo-catalog-contract
//!
//! # Repository Listing (仓库列表查询)
//!
//! 提供 `RepoListing` trait，扩展 `RepoManager` 的列表查询能力。

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::{RepoManager, node_meta};
use crate::models::{DocId, NodeId, NodeMeta, PeerId, RepoType};
use anyhow::{Result, anyhow};

/// 仓库列表查询扩展Trait
pub trait RepoListing {
    /// 列出所有文档
    fn list_docs(&self, repo_type: &RepoType) -> Result<Vec<(DocId, String)>>;

    /// 列出所有节点
    fn list_nodes(&self, repo_type: &RepoType) -> Result<Vec<(NodeId, NodeMeta)>>;

    /// 列出指定 Peer (或本地) 下的所有仓库文件
    fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>>;

    /// 列出当前磁盘上的所有影子库 Peer ID
    fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>>;

    /// 列出至少包含一个可读 shadow repo 的 Peer ID
    fn list_switchable_shadows_on_disk(&self) -> Result<Vec<PeerId>>;
}

impl RepoListing for RepoManager {
    fn list_docs(&self, repo_type: &RepoType) -> Result<Vec<(DocId, String)>> {
        self.run_on_repo_db(repo_type, node_meta::list_file_docs)
    }

    fn list_nodes(&self, repo_type: &RepoType) -> Result<Vec<(NodeId, NodeMeta)>> {
        self.run_on_repo_db(repo_type, node_meta::list_nodes)
    }

    fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>> {
        if let Some(peer_id) = peer_id {
            return self.list_remote_repo_names(peer_id);
        }
        self.refresh_local_repo_catalog()?;
        let target_dir = RepoManager::checked_local_dir_for(&self.ledger_dir, "listing repos")?;

        let mut named = Vec::new();
        for (path, stem) in redb_repo_entries(&target_dir, "listing repos")? {
            let display = if stem == self.local_repo_name {
                self.get_repo_info()?
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

    fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in shadow_peer_dirs(self)? {
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

    fn list_switchable_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in shadow_peer_dirs(self)? {
            if !self.list_remote_repo_names(&peer_id)?.is_empty() {
                peers.push(peer_id);
            }
        }
        peers.sort();
        Ok(peers)
    }
}

fn shadow_peer_dirs(repo: &RepoManager) -> Result<Vec<PeerId>> {
    let remotes_dir = repo.checked_remotes_dir()?;
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
