// crates\core\src\ledger
//! # Repository Listing (仓库列表查询)
//!
//! 提供 `RepoListing` trait，扩展 `RepoManager` 的列表查询能力。

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
        let target_dir = self.ledger_dir.join("local");

        if !target_dir.exists() {
            return Ok(vec![]);
        }

        let mut named = Vec::new();
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("redb") {
                let Some(stem) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(str::to_string)
                else {
                    continue;
                };
                let display = if stem == self.local_repo_name {
                    self.get_repo_info()?.map(|info| info.name)
                } else {
                    RepoManager::read_repo_info_from_path(&path)
                        .map_err(|err| {
                            anyhow!("Broken local repo {} while listing repos: {}", stem, err)
                        })?
                        .map(|info| info.name)
                }
                .unwrap_or_else(|| stem.clone());
                named.push((stem, display));
            }
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
        for peer_id in shadow_peer_dirs(&self.remotes_dir())? {
            let entries = self.scan_remote_repo_entries_without_repair(&peer_id)?;
            if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
                return Err(anyhow!(
                    "Broken shadow peer {} while listing shadows: unreadable repo {}",
                    peer_id,
                    entry.stem
                ));
            }
            if !entries.is_empty() {
                peers.push(peer_id);
            }
        }
        peers.sort();
        Ok(peers)
    }

    fn list_switchable_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let mut peers = Vec::new();
        for peer_id in shadow_peer_dirs(&self.remotes_dir())? {
            if !self.list_remote_repo_names(&peer_id)?.is_empty() {
                peers.push(peer_id);
            }
        }
        peers.sort();
        Ok(peers)
    }
}

fn shadow_peer_dirs(remotes_dir: &std::path::Path) -> Result<Vec<PeerId>> {
    if !remotes_dir.exists() {
        return Ok(vec![]);
    }
    let mut peers = Vec::new();
    for entry in std::fs::read_dir(remotes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name.is_empty() {
            continue;
        }
        peers.push(PeerId::new(name));
    }
    peers.sort();
    Ok(peers)
}
