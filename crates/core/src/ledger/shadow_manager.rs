// crates\core\src\ledger
//! # 影子库管理模块 (Shadow DB Manager)
//!
//! 管理远端影子库 (Store C) 的加载、查询和操作。
//!
//! ## 架构说明
//!
//! 影子库用于存储远端 Peer 的数据副本，实现 Trinity Isolation 架构中的
//! "Receive Only" 隔离策略。每个 Peer 拥有独立的 `.redb` 文件。

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use super::RepoInfo;
use super::RepoManager;
use super::database::relocate_database_path;
use super::ops;
use super::shadow;
use crate::models::{DocId, LedgerEntry, NodeId, PeerId, RepoId, RepoType};
use redb::Database;

type ShadowRepoMap = HashMap<PeerId, HashMap<RepoId, Arc<Database>>>;

impl RepoManager {
    /// 确保指定 Peer 的影子库已加载到内存
    ///
    /// 如果影子库尚未加载，会自动创建或打开对应的 `.redb` 文件。
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    pub fn ensure_shadow_db(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        let db_path = self
            .resolve_remote_repo_entry_by_id(peer_id, *repo_id)?
            .map(|entry| entry.path)
            .unwrap_or_else(|| {
                self.remotes_dir()
                    .join(peer_id.to_filename())
                    .join(format!("{}.redb", repo_id))
            });
        shadow::load_shadow_db(
            &self.remotes_dir(),
            &self.shadow_dbs,
            peer_id,
            repo_id,
            &db_path,
        )
    }

    pub fn ensure_shadow_repo_info(&self, peer_id: &PeerId, info: &RepoInfo) -> Result<()> {
        let desired = self.allocate_remote_repo_path(peer_id, info)?;
        let legacy = self
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid));
        let db_path = match if desired.exists() {
            Some(desired.clone())
        } else if legacy.exists() {
            Some(legacy)
        } else {
            self.resolve_remote_repo_entry_by_id(peer_id, info.uuid)?
                .map(|entry| entry.path)
        } {
            Some(path) if path == desired => path,
            Some(path) => {
                relocate_database_path(&path, &desired)?;
                std::fs::rename(&path, &desired)?;
                self.detach_shadow_repo(peer_id, &info.uuid)?;
                desired
            }
            None => desired,
        };
        shadow::load_shadow_db(
            &self.remotes_dir(),
            &self.shadow_dbs,
            peer_id,
            &info.uuid,
            &db_path,
        )?;
        let dbs = self.read_shadow_dbs()?;
        let db = dbs
            .get(peer_id)
            .and_then(|repos| repos.get(&info.uuid))
            .ok_or_else(|| anyhow!("Shadow DB not loaded for {}/{}", peer_id, info.uuid))?;
        Self::write_repo_info_to_db(db, info)
    }

    /// 列出所有已加载到内存的影子库
    ///
    /// # 返回
    ///
    /// 当前已加载的所有 PeerId 列表
    pub fn list_loaded_shadows(&self) -> Result<Vec<PeerId>> {
        Ok(self.read_shadow_dbs()?.keys().cloned().collect())
    }

    fn detach_shadow_repo(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        let mut dbs = self.write_shadow_dbs()?;
        if let Some(repos) = dbs.get_mut(peer_id) {
            repos.remove(repo_id);
            if repos.is_empty() {
                dbs.remove(peer_id);
            }
        }
        Ok(())
    }

    // Method moved to listing trait

    /// 从指定影子库读取操作（便捷方法）
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    /// * `repo_id` - 仓库 ID
    /// * `doc_id` - 文档 ID
    ///
    /// # 返回
    ///
    /// 该文档在指定影子库中的所有操作记录
    pub fn get_shadow_ops(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.get_ops(&RepoType::Remote(peer_id.clone(), *repo_id), doc_id)
    }

    pub fn get_shadow_structure_ops(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        node_id: NodeId,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            ops::get_structure_ops_for_node_from_db(db, node_id)
        })
    }

    pub(crate) fn read_shadow_dbs(&self) -> Result<RwLockReadGuard<'_, ShadowRepoMap>> {
        self.shadow_dbs
            .read()
            .map_err(|_| anyhow!("Shadow DB registry lock poisoned"))
    }

    pub(crate) fn write_shadow_dbs(&self) -> Result<RwLockWriteGuard<'_, ShadowRepoMap>> {
        self.shadow_dbs
            .write()
            .map_err(|_| anyhow!("Shadow DB registry lock poisoned"))
    }
}

#[cfg(test)]
#[path = "shadow_manager_test.rs"]
mod tests;
