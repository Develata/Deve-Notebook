// crates\core\src\ledger
//! # Repository Listing (仓库列表查询)
//!
//! 提供 `RepoListing` trait，扩展 `RepoManager` 的列表查询能力。

use crate::ledger::{metadata, RepoManager};
use crate::models::{DocId, PeerId, RepoType};
use anyhow::Result;

/// 仓库列表查询扩展Trait
pub trait RepoListing {
    /// 列出所有文档
    fn list_docs(&self, repo_type: &RepoType) -> Result<Vec<(DocId, String)>>;

    /// 列出指定 Peer (或本地) 下的所有仓库文件
    fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>>;

    /// 列出当前磁盘上的所有影子库 Peer ID
    fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>>;
}

impl RepoListing for RepoManager {
    fn list_docs(&self, repo_type: &RepoType) -> Result<Vec<(DocId, String)>> {
        match repo_type {
            RepoType::Local(_) => metadata::list_docs(&self.local_db),
            RepoType::Remote(peer_id, repo_id) => {
                self.ensure_shadow_db(peer_id, repo_id)?;
                let dbs = self.shadow_dbs.read().unwrap();
                let peer_repos = dbs
                    .get(peer_id)
                    .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
                let db = peer_repos.get(repo_id).ok_or_else(|| {
                    anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id)
                })?;
                metadata::list_docs(db)
            }
        }
    }

    fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>> {
        let target_dir = match peer_id {
            None => self.ledger_dir.join("local"),
            Some(pid) => self.remotes_dir().join(pid.to_filename()),
        };

        if !target_dir.exists() {
            return Ok(vec![]);
        }

        let mut repos = Vec::new();
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("redb") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    repos.push(stem.to_string());
                }
            }
        }

        repos.sort();
        Ok(repos)
    }

    fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        let remotes_dir = self.remotes_dir();
        if !remotes_dir.exists() {
            return Ok(vec![]);
        }

        let mut peers = Vec::new();
        for entry in std::fs::read_dir(remotes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    peers.push(PeerId::new(name));
                }
            }
        }
        Ok(peers)
    }
}
