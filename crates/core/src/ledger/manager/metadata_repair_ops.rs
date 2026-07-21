//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract
//!   - 04_repository#repo-catalog-repair-contract
//!
//! # Metadata Repair 操作
//!
//! Invariants:
//! - 这里只暴露 repair / migration / rebuild 允许使用的 projection 映射修补接口。
//! - 业务主路径不得调用这些方法写入 `metadata/path` 真值。

use crate::ledger::RepoManager;
use crate::ledger::metadata;
use anyhow::Result;

impl RepoManager {
    pub fn repair_rename_doc_mapping(&self, old_path: &str, new_path: &str) -> Result<()> {
        self.run_on_primary_local_repo(|db| metadata::rename_doc(db, old_path, new_path))
    }

    pub fn repair_rename_doc_mapping_in_local_repo(
        &self,
        repo_name: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| metadata::rename_doc(db, old_path, new_path))
    }

    pub fn repair_delete_doc_mapping(&self, path: &str) -> Result<()> {
        self.run_on_primary_local_repo(|db| metadata::delete_doc(db, path))
    }

    pub fn repair_delete_doc_mapping_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| metadata::delete_doc(db, path))
    }

    pub fn repair_rename_folder_mapping(&self, old_prefix: &str, new_prefix: &str) -> Result<()> {
        self.run_on_primary_local_repo(|db| metadata::rename_folder(db, old_prefix, new_prefix))
    }

    pub fn repair_rename_folder_mapping_in_local_repo(
        &self,
        repo_name: &str,
        old_prefix: &str,
        new_prefix: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| {
            metadata::rename_folder(db, old_prefix, new_prefix)
        })
    }

    pub fn repair_delete_folder_mapping(&self, prefix: &str) -> Result<usize> {
        self.run_on_primary_local_repo(|db| metadata::delete_folder(db, prefix))
    }

    pub fn repair_delete_folder_mapping_in_local_repo(
        &self,
        repo_name: &str,
        prefix: &str,
    ) -> Result<usize> {
        self.run_on_local_repo(repo_name, |db| metadata::delete_folder(db, prefix))
    }
}
