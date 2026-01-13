//! # 仓库管理器 (Repository Manager)
//!
//! 本模块实现 P2P Git-Flow 架构中的"三位一体隔离" (Trinity Isolation)。
//!
//! ## 架构作用
//!
//! * **Store B (Local Repo)**: 本地权威库 (`local.redb`)，只有本地操作能写入
//! * **Store C (Shadow Repos)**: 远端影子库 (`remotes/peer_X.redb`)，存储远端节点数据
//!
//! ## 核心功能清单
//!
//! - `RepoManager`: 管理本地库和多个影子库的核心结构
//! - `RepoType`: 区分本地库和影子库的枚举
//! - `append_local_op`: 只写入本地库
//! - `append_remote_op`: 只写入指定的影子库
//! - `get_ops`: 支持指定仓库类型读取操作
//!
//! ## 核心必选路径 (Core MUST)
//!
//! 本模块属于 **Core MUST**。所有数据持久化必须通过此模块。

use anyhow::{Result, Context};
use redb::{Database, ReadableTable, ReadableMultimapTable};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use crate::models::{DocId, LedgerEntry, FileNodeId, PeerId};

pub mod schema;
pub mod metadata;
pub mod ops;
pub mod snapshot;
pub mod shadow;
pub mod range;
pub mod source_control;

use self::schema::*;

/// 仓库类型枚举
/// 指定操作的目标仓库。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoType {
    /// 本地权威库 (Store B) - local.redb
    Local,
    /// 远端影子库 (Store C) - remotes/{peer_id}.redb
    Remote(PeerId),
}

/// 仓库管理器 (Repository Manager)
/// 
/// 管理本地唯一的 Local Repo (Store B) 和多个 Shadow Repos (Store C)。
/// 实现 Trinity Isolation 架构中的数据隔离策略。
pub struct RepoManager {
    /// 账本目录根路径
    ledger_dir: PathBuf,
    /// 本地权威库 (local.redb)
    local_db: Database,
    /// 远端影子库集合 (peer_id -> Database) - 懒加载
    shadow_dbs: RwLock<HashMap<PeerId, Database>>,
    /// 快照保留深度
    pub snapshot_depth: usize,
}

impl RepoManager {
    /// 初始化仓库管理器。
    /// 
    /// 创建以下目录结构：
    /// ```text
    /// {ledger_dir}/
    /// ├── local.redb          # 本地权威库
    /// └── remotes/            # 影子库目录
    /// ```
    pub fn init(ledger_dir: impl AsRef<Path>, snapshot_depth: usize) -> Result<Self> {
        let ledger_dir = ledger_dir.as_ref().to_path_buf();
        
        // Create directory structure
        std::fs::create_dir_all(&ledger_dir)
            .with_context(|| format!("Failed to create ledger directory: {:?}", ledger_dir))?;
        
        let remotes_dir = ledger_dir.join("remotes");
        std::fs::create_dir_all(&remotes_dir)
            .with_context(|| format!("Failed to create remotes directory: {:?}", remotes_dir))?;
        
        // Initialize local database
        let local_db_path = ledger_dir.join("local.redb");
        let local_db = Database::create(&local_db_path)
            .with_context(|| format!("Failed to create local database: {:?}", local_db_path))?;
        
        // Initialize tables in local database
        let write_txn = local_db.begin_write()?;
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

        // 初始化 Source Control 表 (暂存区、提交历史)
        source_control::init_tables(&local_db)?;

        Ok(Self { 
            ledger_dir,
            local_db, 
            shadow_dbs: RwLock::new(HashMap::new()),
            snapshot_depth,
        })
    }

    /// 获取账本目录路径。
    pub fn ledger_dir(&self) -> &Path {
        &self.ledger_dir
    }

    /// 获取影子库目录路径。
    pub fn remotes_dir(&self) -> PathBuf {
        self.ledger_dir.join("remotes")
    }

    /// 获取本地数据库的只读事务 (用于高级查询)
    pub fn local_db_read_txn(&self) -> Result<redb::ReadTransaction> {
        Ok(self.local_db.begin_read()?)
    }

    /// 获取本地库指定序列号范围的操作 (用于 P2P 同步增量推送)
    pub fn get_local_ops_in_range(&self, start_seq: u64, end_seq: u64) -> Result<Vec<(u64, LedgerEntry)>> {
        range::get_ops_in_range(&self.local_db, start_seq, end_seq)
    }

    // ========== Shadow DB Management ==========

    /// 确保指定 Peer 的影子库已加载。
    pub fn ensure_shadow_db(&self, peer_id: &PeerId) -> Result<()> {
        shadow::ensure_shadow_db(&self.remotes_dir(), &self.shadow_dbs, peer_id)
    }

    /// 列出所有已加载的影子库。
    pub fn list_loaded_shadows(&self) -> Vec<PeerId> {
        let dbs = self.shadow_dbs.read().unwrap();
        dbs.keys().cloned().collect()
    }

    /// 扫描磁盘上所有影子库文件并返回 PeerId 列表。
    pub fn list_shadows_on_disk(&self) -> Result<Vec<PeerId>> {
        shadow::list_shadows_on_disk(&self.remotes_dir())
    }

    /// 获取指定 Peer 的影子库只读视图。
    /// 
    /// **逻辑**:
    /// 返回 `ShadowRepo` 提供只读访问接口。
    /// 所有写入必须强制通过 `append_remote_op` 进行，以保证单向数据流。
    pub fn get_shadow_repo(&self, peer_id: &PeerId) -> Result<Option<shadow::ShadowRepo<'_>>> {
        self.ensure_shadow_db(peer_id)?;
        
        let dbs = self.shadow_dbs.read().unwrap();
        // We need to check if it exists and return a view
        // Due to lifetime constraints, we need a different approach
        // For now, return None if not found
        if dbs.contains_key(peer_id) {
            // Can't return reference from RwLockReadGuard directly
            // This is a limitation - we'll provide alternative methods instead
            drop(dbs);
            Ok(None) // Placeholder - see get_shadow_ops as alternative
        } else {
            Ok(None)
        }
    }

    /// 从指定影子库读取操作（便捷方法）。
    pub fn get_shadow_ops(&self, peer_id: &PeerId, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        self.get_ops(&RepoType::Remote(peer_id.clone()), doc_id)
    }

    /// 获取指定影子库的全局最大序列号 (用于 Version Vector)。
    pub fn get_shadow_max_seq(&self, peer_id: &PeerId) -> Result<u64> {
        self.ensure_shadow_db(peer_id)?;
        
        let dbs = self.shadow_dbs.read().unwrap();
        let db = dbs.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found for peer: {}", peer_id))?;
        
        range::get_max_seq(db)
    }

    /// 获取指定影子库指定序列号范围的操作 (用于 P2P 同步增量拉取)。
    pub fn get_shadow_ops_in_range(&self, peer_id: &PeerId, start_seq: u64, end_seq: u64) -> Result<Vec<(u64, LedgerEntry)>> {
        self.ensure_shadow_db(peer_id)?;
        
        let dbs = self.shadow_dbs.read().unwrap();
        let db = dbs.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found for peer: {}", peer_id))?;
        
        range::get_ops_in_range(db, start_seq, end_seq)
    }

    // ========== Snapshot Operations ==========

    /// Save a snapshot for a document (Local DB only).
    pub fn save_snapshot(&self, doc_id: DocId, seq: u64, content: &str) -> Result<()> {
        snapshot::save_snapshot(&self.local_db, doc_id, seq, content, self.snapshot_depth)
    }

    // ========== Path/DocId Mapping (Local DB Only) ==========

    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        metadata::get_docid(&self.local_db, path)
    }

    pub fn create_docid(&self, path: &str) -> Result<DocId> {
        metadata::create_docid(&self.local_db, path)
    }

    pub fn get_path_by_docid(&self, doc_id: DocId) -> Result<Option<String>> {
         metadata::get_path_by_docid(&self.local_db, doc_id)
    }

    pub fn get_docid_by_inode(&self, inode: &FileNodeId) -> Result<Option<DocId>> {
        metadata::get_docid_by_inode(&self.local_db, inode)
    }

    pub fn bind_inode(&self, inode: &FileNodeId, doc_id: DocId) -> Result<()> {
        metadata::bind_inode(&self.local_db, inode, doc_id)
    }

    pub fn rename_doc(&self, old_path: &str, new_path: &str) -> Result<()> {
        metadata::rename_doc(&self.local_db, old_path, new_path)
    }

    pub fn delete_doc(&self, path: &str) -> Result<()> {
        metadata::delete_doc(&self.local_db, path)
    }

    pub fn rename_folder(&self, old_prefix: &str, new_prefix: &str) -> Result<()> {
        metadata::rename_folder(&self.local_db, old_prefix, new_prefix)
    }

    pub fn delete_folder(&self, prefix: &str) -> Result<usize> {
        metadata::delete_folder(&self.local_db, prefix)
    }

    pub fn list_docs(&self) -> Result<Vec<(DocId, String)>> {
        metadata::list_docs(&self.local_db)
    }

    // ========== Operations (Append/Read) ==========

    /// 追加操作到本地库 (Store B)。
    /// 
    /// **权限**:
    /// Local Write Only - 仅接受本地用户的操作。
    pub fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        ops::append_op_to_db(&self.local_db, entry)
    }

    /// 追加操作到指定远端的影子库 (Store C)。
    /// 
    /// **权限**:
    /// Remote Write Only - 仅接受来自指定 Peer 的操作。
    pub fn append_remote_op(&self, peer_id: &PeerId, entry: &LedgerEntry) -> Result<u64> {
        self.ensure_shadow_db(peer_id)?;
        
        let dbs = self.shadow_dbs.read().unwrap();
        let db = dbs.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found for peer: {}", peer_id))?;
        
        ops::append_op_to_db(db, entry)
    }

    /// 兼容方法：追加操作到本地库（保持原有 API 兼容性）。
    #[deprecated(note = "Use append_local_op instead for clarity")]
    pub fn append_op(&self, entry: &LedgerEntry) -> Result<u64> {
        self.append_local_op(entry)
    }

    /// 从指定仓库读取操作。
    pub fn get_ops(&self, repo_type: &RepoType, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        match repo_type {
            RepoType::Local => ops::get_ops_from_db(&self.local_db, doc_id),
            RepoType::Remote(peer_id) => {
                self.ensure_shadow_db(peer_id)?;
                let dbs = self.shadow_dbs.read().unwrap();
                let db = dbs.get(peer_id)
                    .ok_or_else(|| anyhow::anyhow!("Shadow DB not found for peer: {}", peer_id))?;
                ops::get_ops_from_db(db, doc_id)
            }
        }
    }

    /// 从本地库读取操作（便捷方法）。
    pub fn get_local_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        self.get_ops(&RepoType::Local, doc_id)
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

    /// 创建提交
    pub fn create_commit(&self, message: &str) -> Result<crate::source_control::CommitInfo> {
        source_control::create_commit(&self.local_db, message)
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<crate::source_control::CommitInfo>> {
        source_control::list_commits(&self.local_db, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::TempDir;

    #[test]
    fn test_repo_manager_init() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let ledger_dir = tmp_dir.path().join("ledger");
        
        let repo = RepoManager::init(&ledger_dir, 10)?;
        
        // Verify directory structure
        assert!(ledger_dir.exists());
        assert!(ledger_dir.join("local.redb").exists());
        assert!(ledger_dir.join("remotes").exists());
        
        Ok(())
    }

    #[test]
    fn test_local_and_shadow_isolation() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let ledger_dir = tmp_dir.path().join("ledger");
        let repo = RepoManager::init(&ledger_dir, 10)?;
        
        let doc_id = DocId::new();
        let peer_id = PeerId::new("peer_mobile");
        
        // Write to local
        let local_entry = LedgerEntry {
            doc_id,
            op: crate::models::Op::Insert { pos: 0, content: "local content".to_string() },
            timestamp: 1000,
        };
        repo.append_local_op(&local_entry)?;
        
        // Write to shadow
        let remote_entry = LedgerEntry {
            doc_id,
            op: crate::models::Op::Insert { pos: 0, content: "remote content".to_string() },
            timestamp: 2000,
        };
        repo.append_remote_op(&peer_id, &remote_entry)?;
        
        // Verify isolation
        let local_ops = repo.get_ops(&RepoType::Local, doc_id)?;
        assert_eq!(local_ops.len(), 1);
        
        let remote_ops = repo.get_ops(&RepoType::Remote(peer_id.clone()), doc_id)?;
        assert_eq!(remote_ops.len(), 1);
        
        // Verify shadow db file exists
        let shadow_path = ledger_dir.join("remotes").join("peer_mobile.redb");
        assert!(shadow_path.exists());
        
        Ok(())
    }

    #[test]
    fn test_snapshot_pruning() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let ledger_dir = tmp_dir.path().join("ledger");
        
        let repo = RepoManager::init(&ledger_dir, 2)?;
        let doc_id = DocId::new();
        
        // Save 3 snapshots
        repo.save_snapshot(doc_id, 1, "Snap 1")?;
        repo.save_snapshot(doc_id, 2, "Snap 2")?;
        repo.save_snapshot(doc_id, 3, "Snap 3")?; // This should prune seq 1
        
        // Verify pruning
        let read_txn = repo.local_db.begin_read()?;
        let index = read_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let data = read_txn.open_table(SNAPSHOT_DATA)?;
        
        let mut seqs = Vec::new();
        for item in index.get(doc_id.as_u128())? {
            seqs.push(item?.value());
        }
        seqs.sort();
        
        assert_eq!(seqs, vec![2, 3], "Snapshot index should only contain 2 and 3");
        assert!(data.get(1)?.is_none(), "Snapshot 1 data should be removed");
        assert!(data.get(2)?.is_some(), "Snapshot 2 data should exist");
        assert!(data.get(3)?.is_some(), "Snapshot 3 data should exist");
        
        Ok(())
    }
}
