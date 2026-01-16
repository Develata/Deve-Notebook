//! # 仓库管理器初始化模块 (RepoManager Initialization)
//!
//! 处理 RepoManager 的初始化逻辑，包括目录结构创建和数据库表初始化。
//!
//! ## 目录结构
//!
//! ```text
//! {ledger_dir}/
//! ├── local.redb          # 本地权威库 (Store B)
//! └── remotes/            # 影子库目录 (Store C)
//!     ├── peer_a.redb
//!     └── peer_b.redb
//! ```

use anyhow::{Context, Result};
use redb::Database;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use super::schema::*;
use super::source_control;
use super::RepoManager;

/// 初始化 RepoManager 实例
///
/// 创建账本目录结构，打开/创建本地数据库，并初始化所有必需的表。
///
/// # 参数
///
/// * `ledger_dir` - 账本根目录路径
/// * `snapshot_depth` - 快照保留深度（超出部分会被裁剪）
///
/// # 错误
///
/// 当目录创建或数据库操作失败时返回错误。
///
/// # 示例
///
/// ```ignore
/// use deve_core::ledger::RepoManager;
///
/// let repo = RepoManager::init("./data/ledger", 10)?;
/// ```
pub fn init(ledger_dir: impl AsRef<Path>, snapshot_depth: usize) -> Result<RepoManager> {
    let ledger_dir = ledger_dir.as_ref().to_path_buf();
    
    // 创建目录结构
    std::fs::create_dir_all(&ledger_dir)
        .with_context(|| format!("无法创建账本目录: {:?}", ledger_dir))?;
    
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)
        .with_context(|| format!("无法创建本地库目录: {:?}", local_dir))?;

    let remotes_dir = ledger_dir.join("remotes");
    std::fs::create_dir_all(&remotes_dir)
        .with_context(|| format!("无法创建远端目录: {:?}", remotes_dir))?;
    
    // 初始化本地数据库 (Default Local Repo)
    // TODO: support multi-repo init? For now, we create a default one.
    // If we want to support existing single-file, we might need migration.
    // But for new structure:
    let local_db_path = local_dir.join("default.redb");
    let local_db = Database::create(&local_db_path)
        .with_context(|| format!("无法创建本地数据库: {:?}", local_db_path))?;
    
    // 初始化核心表
    init_core_tables(&local_db)?;
    
    // 初始化 Source Control 表
    source_control::init_tables(&local_db)?;

    Ok(RepoManager { 
        ledger_dir,
        local_db, 
        shadow_dbs: RwLock::new(HashMap::new()),
        snapshot_depth,
    })
}

/// 初始化本地数据库的核心表
///
/// 包括:
/// - `DOCID_TO_PATH`: DocId -> 文件路径 映射
/// - `PATH_TO_DOCID`: 文件路径 -> DocId 映射
/// - `INODE_TO_DOCID`: Inode -> DocId 映射 (用于重命名检测)
/// - `LEDGER_OPS`: 操作日志表
/// - `DOC_OPS`: 文档操作索引
/// - `SNAPSHOT_INDEX`: 快照索引
/// - `SNAPSHOT_DATA`: 快照数据
fn init_core_tables(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;
    }
    write_txn.commit()?;
    Ok(())
}
