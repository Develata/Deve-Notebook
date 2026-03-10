// crates/core/src/ledger/manager/metadata_ops.rs
//! # Path/DocId 查询与绑定
//!
//! 实现 `RepoManager` 的只读查询与 identity 绑定方法。
//! repair/rebuild 使用的映射直写接口已拆到 `metadata_repair_ops`。

use crate::ledger::RepoManager;
use crate::ledger::{doc_lookup, metadata};
use crate::models::DocId;
use anyhow::Result;

impl RepoManager {
    /// 根据路径获取 DocId
    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        doc_lookup::resolve_doc_id(&self.local_db, path)
    }

    /// 创建新的 DocId
    pub fn create_docid(&self, path: &str) -> Result<DocId> {
        metadata::create_docid(&self.local_db, path)
    }

    pub fn create_docid_in_local_repo(&self, repo_name: &str, path: &str) -> Result<DocId> {
        self.run_on_local_repo(repo_name, |db| metadata::create_docid(db, path))
    }

    /// 根据 DocId 获取路径
    pub fn get_path_by_docid(&self, doc_id: DocId) -> Result<Option<String>> {
        metadata::get_path_by_docid(&self.local_db, doc_id)
    }

    pub fn get_path_by_docid_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
    ) -> Result<Option<String>> {
        self.run_on_local_repo(repo_name, |db| metadata::get_path_by_docid(db, doc_id))
    }

    /// 根据 Inode 获取 DocId
    pub fn get_docid_by_inode(&self, inode: &crate::models::FileNodeId) -> Result<Option<DocId>> {
        metadata::get_docid_by_inode(&self.local_db, inode)
    }

    pub fn get_docid_by_inode_in_local_repo(
        &self,
        repo_name: &str,
        inode: &crate::models::FileNodeId,
    ) -> Result<Option<DocId>> {
        self.run_on_local_repo(repo_name, |db| metadata::get_docid_by_inode(db, inode))
    }

    pub fn get_docid_in_local_repo(&self, repo_name: &str, path: &str) -> Result<Option<DocId>> {
        self.run_on_local_repo(repo_name, |db| doc_lookup::resolve_doc_id(db, path))
    }

    /// 绑定 Inode 到 DocId
    pub fn bind_inode(&self, inode: &crate::models::FileNodeId, doc_id: DocId) -> Result<()> {
        metadata::bind_inode(&self.local_db, inode, doc_id)
    }

    pub fn bind_inode_in_local_repo(
        &self,
        repo_name: &str,
        inode: &crate::models::FileNodeId,
        doc_id: DocId,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| metadata::bind_inode(db, inode, doc_id))
    }
}
