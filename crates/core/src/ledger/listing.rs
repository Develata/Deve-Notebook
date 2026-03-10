// crates\core\src\ledger
//! # Repository Listing (仓库列表查询)
//!
//! 提供 `RepoListing` trait，扩展 `RepoManager` 的列表查询能力。

use crate::ledger::{RepoManager, metadata, node_meta};
use crate::models::{DocId, NodeId, NodeMeta, PeerId, RepoType};
use anyhow::Result;

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
}

impl RepoListing for RepoManager {
    fn list_docs(&self, repo_type: &RepoType) -> Result<Vec<(DocId, String)>> {
        self.run_on_repo_db(repo_type, metadata::list_docs)
    }

    fn list_nodes(&self, repo_type: &RepoType) -> Result<Vec<(NodeId, NodeMeta)>> {
        self.run_on_repo_db(repo_type, node_meta::list_nodes)
    }

    fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>> {
        if let Some(peer_id) = peer_id {
            return self.list_remote_repo_names(peer_id);
        }
        let target_dir = self.ledger_dir.join("local");

        if !target_dir.exists() {
            return Ok(vec![]);
        }

        let mut repos = Vec::new();
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("redb")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                repos.push(stem.to_string());
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
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
            {
                peers.push(PeerId::new(name));
            }
        }
        Ok(peers)
    }
}
