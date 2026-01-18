// crates\core\src\ledger
//! # 影子库管理模块 (Shadow DB Manager)
//!
//! 管理远端影子库 (Store C) 的加载、查询和操作。
//!
//! ## 架构说明
//!
//! 影子库用于存储远端 Peer 的数据副本，实现 Trinity Isolation 架构中的
//! "Receive Only" 隔离策略。每个 Peer 拥有独立的 `.redb` 文件。

use anyhow::Result;

use super::RepoManager;
use super::ops;
use super::range;
use super::shadow;
use crate::models::{DocId, LedgerEntry, PeerId, RepoId, RepoType};

impl RepoManager {
    /// 确保指定 Peer 的影子库已加载到内存
    ///
    /// 如果影子库尚未加载，会自动创建或打开对应的 `.redb` 文件。
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    pub fn ensure_shadow_db(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<()> {
        shadow::ensure_shadow_db(&self.remotes_dir(), &self.shadow_dbs, peer_id, repo_id)
    }

    /// 列出所有已加载到内存的影子库
    ///
    /// # 返回
    ///
    /// 当前已加载的所有 PeerId 列表
    pub fn list_loaded_shadows(&self) -> Vec<PeerId> {
        let dbs = self.shadow_dbs.read().unwrap();
        dbs.keys().cloned().collect()
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

    /// 获取指定影子库的全局最大序列号
    ///
    /// 用于 Version Vector 计算和增量同步。
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    /// * `repo_id` - 仓库 ID
    ///
    /// # 返回
    ///
    /// 该影子库中的最大操作序列号
    pub fn get_shadow_max_seq(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<u64> {
        self.ensure_shadow_db(peer_id, repo_id)?;

        let dbs = self.shadow_dbs.read().unwrap();
        let peer_repos = dbs
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
        let db = peer_repos
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id))?;

        range::get_max_seq(db)
    }

    /// 获取指定影子库指定序列号范围的操作
    ///
    /// 用于 P2P 同步中的增量拉取。
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    /// * `repo_id` - 仓库 ID
    /// * `start_seq` - 起始序列号（包含）
    /// * `end_seq` - 结束序列号（包含）
    ///
    /// # 返回
    ///
    /// 指定范围内的所有操作记录
    pub fn get_shadow_ops_in_range(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.ensure_shadow_db(peer_id, repo_id)?;

        let dbs = self.shadow_dbs.read().unwrap();
        let peer_repos = dbs
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
        let db = peer_repos
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id))?;

        range::get_ops_in_range(db, start_seq, end_seq)
    }

    /// 追加操作到指定远端的影子库 (Store C)
    ///
    /// **权限**: Remote Write Only - 仅接受来自指定 Peer 的操作。
    ///
    /// # 参数
    ///
    /// * `peer_id` - 远端 Peer 的唯一标识
    /// * `repo_id` - 仓库 ID
    /// * `entry` - 要追加的操作记录
    ///
    /// # 返回
    ///
    /// 新操作的序列号
    pub fn append_remote_op(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        self.ensure_shadow_db(peer_id, repo_id)?;

        let dbs = self.shadow_dbs.read().unwrap();
        let peer_repos = dbs
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
        let db = peer_repos
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id))?;

        ops::append_op_to_db(db, entry)
    }
}
