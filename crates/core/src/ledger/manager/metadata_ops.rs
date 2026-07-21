// crates/core/src/ledger/manager/metadata_ops.rs
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/index#internal-path-normalization
//!
//! # Path/DocId 查询与绑定
//!
//! 实现 `RepoManager` 的只读查询与 identity 绑定方法。
//! repair/rebuild 使用的映射直写接口已拆到 `metadata_repair_ops`。

use crate::ledger::RepoManager;
use crate::ledger::{doc_lookup, inode_index, node_meta};
use crate::models::{DocId, NodeMeta};
use crate::utils::fs::checked_exists;
use anyhow::Result;

impl RepoManager {
    /// 根据路径获取 DocId
    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        self.run_on_primary_local_repo(|db| doc_lookup::resolve_doc_id(db, path))
    }

    pub fn get_file_meta_for_doc(&self, doc_id: DocId) -> Result<Option<NodeMeta>> {
        self.run_on_primary_local_repo(|db| node_meta::file_meta_for_doc(db, doc_id))
    }

    pub fn get_file_meta_for_doc_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
    ) -> Result<Option<NodeMeta>> {
        self.run_on_local_repo(repo_name, |db| node_meta::file_meta_for_doc(db, doc_id))
    }

    /// 根据 Inode 获取 DocId
    pub fn get_docid_by_inode(&self, inode: &crate::models::FileNodeId) -> Result<Option<DocId>> {
        self.run_on_primary_local_repo(|db| inode_index::get_docid(db, inode))
    }

    pub fn get_docid_by_inode_in_local_repo(
        &self,
        repo_name: &str,
        inode: &crate::models::FileNodeId,
    ) -> Result<Option<DocId>> {
        self.run_on_local_repo(repo_name, |db| inode_index::get_docid(db, inode))
    }

    /// 运行时主链的 tracked doc 查询：
    /// 先走 `path -> node_id -> node_meta.doc_id`，避免将旧 path mapping 当作权威真值。
    pub fn get_tracked_docid_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<Option<DocId>> {
        self.run_on_local_repo(repo_name, |db| {
            let Some(node_id) = node_meta::get_node_id(db, path)? else {
                return Ok(None);
            };
            Ok(node_meta::get_node_meta(db, node_id)?.and_then(|meta| meta.doc_id))
        })
    }

    /// 绑定 Inode 到 DocId
    pub fn bind_inode(&self, inode: &crate::models::FileNodeId, doc_id: DocId) -> Result<()> {
        self.run_on_primary_local_repo(|db| inode_index::bind_docid(db, inode, doc_id))
    }

    pub fn bind_inode_in_local_repo(
        &self,
        repo_name: &str,
        inode: &crate::models::FileNodeId,
        doc_id: DocId,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| inode_index::bind_docid(db, inode, doc_id))
    }

    pub fn bind_workspace_inode_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        doc_id: DocId,
    ) -> Result<()> {
        let file_path = self.local_repo_workspace_path(repo_name, path)?;
        if !checked_exists(&file_path, "workspace path while binding inode")? {
            return Ok(());
        }
        let file_id = file_id::get_file_id(&file_path)?;
        use crate::utils::hash::StableHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = StableHasher::new();
        file_id.hash(&mut hasher);
        let inode = crate::models::FileNodeId {
            id: hasher.finish() as u128,
        };
        self.bind_inode_in_local_repo(repo_name, &inode, doc_id)
    }
}
