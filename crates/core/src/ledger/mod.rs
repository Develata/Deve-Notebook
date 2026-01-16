//! # 仓库管理器 (Repository Manager)
//!
//! 本模块实现 P2P Git-Flow 架构中的"三位一体隔离" (Trinity Isolation)。
//!
//! ## 架构作用
//!
//! * **Store B (Local Repo)**: 本地权威库 (`local.redb`)，只有本地操作能写入
//! * **Store C (Shadow Repos)**: 远端影子库 (`remotes/peer_X.redb`)，存储远端节点数据
//!
//! ## 模块结构
//!
//! - `repo_type`: 仓库类型枚举 (Local/Remote)
//! - `init`: 初始化逻辑
//! - `shadow_manager`: Shadow DB 管理
//! - `schema`: 数据库表定义
//! - `metadata`: Path/DocId 映射
//! - `ops`: 操作日志读写
//! - `snapshot`: 快照管理
//! - `shadow`: Shadow 库底层实现
//! - `range`: 范围查询
//! - `source_control`: 版本控制集成
//!
//! ## 核心必选路径 (Core MUST)
//!
//! 本模块属于 **Core MUST**。所有数据持久化必须通过此模块。

use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::{DocId, LedgerEntry, PeerId, RepoType, RepoId};

// ========== 子模块声明 ==========

pub mod init;
mod shadow_manager;
pub mod schema;
pub mod metadata;
pub mod ops;
pub mod snapshot;
pub mod shadow;
pub mod range;
pub mod source_control;
pub mod listing;

// ========== 公开导出 ==========

pub use self::schema::*;

#[cfg(test)]
mod tests;

/// 仓库管理器 (Repository Manager)
/// 
/// 管理本地唯一的 Local Repo (Store B) 和多个 Shadow Repos (Store C)。
/// 实现 Trinity Isolation 架构中的数据隔离策略。
pub struct RepoManager {
    /// 账本目录根路径
    pub(crate) ledger_dir: PathBuf,
    /// 本地权威库 (local.redb)
    pub(crate) local_db: Database,
    /// 远端影子库集合 (peer_id -> repo_id -> Database) - 懒加载
    pub(crate) shadow_dbs: RwLock<HashMap<PeerId, HashMap<RepoId, Database>>>,
    /// 快照保留深度
    pub snapshot_depth: usize,
}

impl RepoManager {
    /// 初始化仓库管理器
    ///
    /// 详细文档见 `init` 模块。
    /// 初始化 RepoManager
    pub fn init(ledger_dir: impl AsRef<Path>, snapshot_depth: usize, repo_name: Option<&str>) -> Result<Self> {
        init::init(ledger_dir, snapshot_depth, repo_name)
    }

    /// 获取账本目录路径
    pub fn ledger_dir(&self) -> &Path {
        &self.ledger_dir
    }

    /// 获取影子库目录路径
    pub fn remotes_dir(&self) -> PathBuf {
        self.ledger_dir.join("remotes")
    }

    /// 获取本地数据库的只读事务 (用于高级查询)
    pub fn local_db_read_txn(&self) -> Result<redb::ReadTransaction> {
        Ok(self.local_db.begin_read()?)
    }

    /// 获取本地库指定序列号范围的操作 (用于 P2P 同步增量推送)
    pub fn get_local_ops_in_range(&self, _repo_id: &RepoId, start_seq: u64, end_seq: u64) -> Result<Vec<(u64, LedgerEntry)>> {
        // TODO: support multi local repos. For now we use the active local_db.
        range::get_ops_in_range(&self.local_db, start_seq, end_seq)
    }

    // ========== Snapshot Operations ==========

    /// 保存文档快照 (仅限本地库)
    pub fn save_snapshot(&self, doc_id: DocId, seq: u64, content: &str) -> Result<()> {
        snapshot::save_snapshot(&self.local_db, doc_id, seq, content, self.snapshot_depth)
    }

    // ========== Path/DocId Mapping ==========

    /// 根据路径获取 DocId
    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        metadata::get_docid(&self.local_db, path)
    }

    /// 创建新的 DocId
    pub fn create_docid(&self, path: &str) -> Result<DocId> {
        metadata::create_docid(&self.local_db, path)
    }

    /// 根据 DocId 获取路径
    pub fn get_path_by_docid(&self, doc_id: DocId) -> Result<Option<String>> {
         metadata::get_path_by_docid(&self.local_db, doc_id)
    }

    /// 根据 Inode 获取 DocId
    pub fn get_docid_by_inode(&self, inode: &crate::models::FileNodeId) -> Result<Option<DocId>> {
        metadata::get_docid_by_inode(&self.local_db, inode)
    }

    /// 绑定 Inode 到 DocId
    pub fn bind_inode(&self, inode: &crate::models::FileNodeId, doc_id: DocId) -> Result<()> {
        metadata::bind_inode(&self.local_db, inode, doc_id)
    }

    /// 重命名文档
    pub fn rename_doc(&self, old_path: &str, new_path: &str) -> Result<()> {
        metadata::rename_doc(&self.local_db, old_path, new_path)
    }

    /// 删除文档
    pub fn delete_doc(&self, path: &str) -> Result<()> {
        metadata::delete_doc(&self.local_db, path)
    }

    /// 重命名文件夹
    pub fn rename_folder(&self, old_prefix: &str, new_prefix: &str) -> Result<()> {
        metadata::rename_folder(&self.local_db, old_prefix, new_prefix)
    }

    /// 删除文件夹
    pub fn delete_folder(&self, prefix: &str) -> Result<usize> {
        metadata::delete_folder(&self.local_db, prefix)
    }

    // ========== Operations (Append/Read) ==========

    /// 追加操作到本地库 (Store B)
    ///
    /// **权限**: Local Write Only - 仅接受本地用户的操作。
    pub fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        ops::append_op_to_db(&self.local_db, entry)
    }

    /// 兼容方法：追加操作到本地库
    #[deprecated(note = "请使用 append_local_op 以保持语义清晰")]
    pub fn append_op(&self, entry: &LedgerEntry) -> Result<u64> {
        self.append_local_op(entry)
    }

    /// 从指定仓库读取操作
    pub fn get_ops(&self, repo_type: &RepoType, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        match repo_type {
            RepoType::Local(_) => ops::get_ops_from_db(&self.local_db, doc_id),
            RepoType::Remote(peer_id, repo_id) => {
                self.ensure_shadow_db(peer_id, repo_id)?; 
                let dbs = self.shadow_dbs.read().unwrap();
                let peer_repos = dbs.get(peer_id)
                    .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
                let db = peer_repos.get(repo_id)
                    .ok_or_else(|| anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id))?;
                ops::get_ops_from_db(db, doc_id)
            }
        }
    }

    /// 从本地库读取操作（便捷方法）
    pub fn get_local_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        // 使用默认 UUID (nil) 代表当前活动的本地库
        self.get_ops(&RepoType::Local(uuid::Uuid::nil()), doc_id)
    }

    // ========== Source Control Operations ==========

    /// 暂存指定文件
    pub fn stage_file(&self, path: &str) -> Result<()> {
        source_control::stage_file(&self.local_db, path)
    }

    /// 取消暂存指定文件
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        source_control::unstage_file(&self.local_db, path)
    }

    /// 获取已暂存文件列表
    pub fn list_staged(&self) -> Result<Vec<crate::source_control::ChangeEntry>> {
        source_control::list_staged(&self.local_db)
    }

    /// 创建提交 (保存快照)
    pub fn create_commit_with_snapshots<F>(
        &self,
        message: &str,
        get_content: F,
    ) -> Result<crate::source_control::CommitInfo>
    where
        F: Fn(&str) -> Option<(DocId, String)>,
    {
        source_control::create_commit(&self.local_db, message, get_content)
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<crate::source_control::CommitInfo>> {
        source_control::list_commits(&self.local_db, limit)
    }

    /// 获取文档的已提交内容 (用于 Diff)
    pub fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        source_control::get_committed_content(&self.local_db, doc_id)
    }

    /// 检测单个文档的变更状态
    pub fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<crate::source_control::ChangeStatus> {
        source_control::detect_change(committed, current)
    }
}
