// crates/core/src/ledger/manager/metadata_ops.rs
//! # Path/DocId 映射操作
//!
//! 实现 `RepoManager` 的路径与 DocId 映射相关方法。

use crate::ledger::RepoManager;
use crate::ledger::metadata;
use crate::models::DocId;
use anyhow::Result;

impl RepoManager {
    /// 根据路径获取 DocId
    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        metadata::get_docid(&self.local_db, path)
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
        self.run_on_local_repo(repo_name, |db| metadata::get_docid(db, path))
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

    /// 重命名文档
    pub fn rename_doc(&self, old_path: &str, new_path: &str) -> Result<()> {
        metadata::rename_doc(&self.local_db, old_path, new_path)
    }

    pub fn rename_doc_in_local_repo(
        &self,
        repo_name: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| metadata::rename_doc(db, old_path, new_path))
    }

    /// 删除文档
    pub fn delete_doc(&self, path: &str) -> Result<()> {
        metadata::delete_doc(&self.local_db, path)
    }

    pub fn delete_doc_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| metadata::delete_doc(db, path))
    }

    /// 重命名文件夹
    pub fn rename_folder(&self, old_prefix: &str, new_prefix: &str) -> Result<()> {
        metadata::rename_folder(&self.local_db, old_prefix, new_prefix)
    }

    pub fn rename_folder_in_local_repo(
        &self,
        repo_name: &str,
        old_prefix: &str,
        new_prefix: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| {
            metadata::rename_folder(db, old_prefix, new_prefix)
        })
    }

    /// 删除文件夹
    pub fn delete_folder(&self, prefix: &str) -> Result<usize> {
        metadata::delete_folder(&self.local_db, prefix)
    }

    pub fn delete_folder_in_local_repo(&self, repo_name: &str, prefix: &str) -> Result<usize> {
        self.run_on_local_repo(repo_name, |db| metadata::delete_folder(db, prefix))
    }
}
