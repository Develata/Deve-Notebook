// crates\core\src\ledger
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 04_repository#repo-catalog-contract
//!
//! # Repository Listing (仓库列表查询)
//!
//! 提供 `RepoListing` trait，扩展 `RepoManager` 的列表查询能力。

use crate::ledger::{RepoManager, node_meta};
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
        self.repo_catalog_runtime().list_repos(peer_id)
    }

    fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        self.repo_catalog_runtime().list_shadows_on_disk()
    }

    fn list_switchable_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        self.repo_catalog_runtime()
            .list_switchable_shadows_on_disk()
    }
}
