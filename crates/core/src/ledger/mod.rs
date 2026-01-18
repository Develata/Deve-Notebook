// crates/core/src/ledger/mod.rs
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
//! - `schema`: 数据库表定义
//! - `init`: 初始化逻辑
//! - `metadata`: Path/DocId 映射
//! - `ops`: 操作日志读写
//! - `snapshot`: 快照管理
//! - `range`: 范围查询
//! - `shadow`: Shadow 库底层实现
//! - `shadow_manager`: Shadow DB 管理
//! - `source_control`: 版本控制集成
//! - `listing`: 文档列表
//! - `merge`: 合并引擎
//! - `manager`: RepoManager 实现分布模块

use anyhow::Result;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::{LedgerEntry, PeerId, RepoId};

// ========== 子模块声明 ==========

pub mod init;
pub mod listing;
mod manager;
pub mod merge;
pub mod metadata;
pub mod ops;
pub mod range;
pub mod schema;
pub mod shadow;
mod shadow_manager;
pub mod snapshot;
pub mod source_control;

// ========== 公开导出 ==========

pub use self::schema::*;

#[cfg(test)]
mod tests;

/// 仓库元数据信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// 仓库唯一标识
    pub uuid: RepoId,
    /// 仓库名称 (Human Readable)
    pub name: String,
    /// 仓库 URL (唯一逻辑标识) - 可选
    pub url: Option<String>,
}

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
    pub fn init(
        ledger_dir: impl AsRef<Path>,
        snapshot_depth: usize,
        repo_name: Option<&str>,
        repo_url: Option<&str>,
    ) -> Result<Self> {
        init::init(ledger_dir, snapshot_depth, repo_name, repo_url)
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
    pub fn get_local_ops_in_range(
        &self,
        _repo_id: &RepoId,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        range::get_ops_in_range(&self.local_db, start_seq, end_seq)
    }

    /// 获取本地仓库的元数据信息 (UUID, Name, URL)
    pub fn get_repo_info(&self) -> Result<Option<RepoInfo>> {
        let read_txn = self.local_db.begin_read()?;
        let table = read_txn.open_table(REPO_METADATA)?;
        if let Some(guard) = table.get(&0)? {
            let value = guard.value();
            let info: RepoInfo = bincode::deserialize(value)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}
