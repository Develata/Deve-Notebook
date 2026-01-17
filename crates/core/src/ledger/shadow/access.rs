// crates\core\src\ledger\shadow
//! # 影子库访问层 (Shadow Access)
//!
//! **架构作用**:
//! 提供对单个影子库的只读视图封装。
//! 复用 `range.rs` 的查询能力。
//!
//! **核心功能清单**:
//! - `ShadowRepo`: 对 redb::Database 的只读包装。
//! - `get_ops`: 获取操作。
//! - `get_global_max_seq`: 获取最大序列号 (Delegates to range module).
//!
//! **类型**: Core MUST (核心必选)

use anyhow::Result;
use redb::Database;
use crate::models::{DocId, LedgerEntry, PeerId, RepoId};
use crate::ledger::{ops, range};
use crate::ledger::schema::*;


/// 影子库 (Shadow Repository)
/// 
/// 封装单个远端 Peer 的影子数据库，提供只读访问接口。
/// 所有写入操作必须通过 `RepoManager::append_remote_op` 进行，
/// 此结构仅提供查询能力。
pub struct ShadowRepo<'a> {
    /// Peer ID
    pub peer_id: PeerId,
    /// Repo ID
    pub repo_id: RepoId,
    /// 数据库引用 (只读访问)
    db: &'a Database,
}

impl<'a> ShadowRepo<'a> {
    /// 创建影子库的只读视图
    pub(crate) fn new(peer_id: PeerId, repo_id: RepoId, db: &'a Database) -> Self {
        Self { peer_id, repo_id, db }
    }

    /// 获取指定文档的所有操作 (只读)
    pub fn get_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        ops::get_ops_from_db(self.db, doc_id)
    }

    /// 获取指定文档的操作数量
    pub fn count_ops(&self, doc_id: DocId) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_multimap_table(DOC_OPS)?;
        
        let mut count = 0u64;
        for _ in table.get(doc_id.as_u128())? {
            count += 1;
        }
        Ok(count)
    }

    /// 获取指定文档的最大序列号
    pub fn get_max_seq(&self, doc_id: DocId) -> Result<Option<u64>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_multimap_table(DOC_OPS)?;
        
        let mut max_seq: Option<u64> = None;
        for item in table.get(doc_id.as_u128())? {
            let seq = item?.value();
            max_seq = Some(max_seq.map_or(seq, |m| m.max(seq)));
        }
        Ok(max_seq)
    }

    /// 获取全局最大序列号 (用于 Version Vector)
    pub fn get_global_max_seq(&self) -> Result<u64> {
        range::get_max_seq(self.db)
    }

    /// 获取指定序列号范围的操作 (用于 P2P 同步增量拉取)
    pub fn get_ops_in_range(&self, start_seq: u64, end_seq: u64) -> Result<Vec<(u64, LedgerEntry)>> {
        range::get_ops_in_range(self.db, start_seq, end_seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::RwLock;
    use std::collections::HashMap;
    use crate::models::Op;
    use crate::ledger::schema::{LEDGER_OPS, DOC_OPS};
    use crate::ledger::shadow::management::ensure_shadow_db;
    use uuid::Uuid;

    #[test]
    fn test_shadow_repo_read_only_access() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let remotes_dir = tmp_dir.path().join("remotes");
        std::fs::create_dir_all(&remotes_dir)?;
        
        let shadow_dbs: RwLock<HashMap<PeerId, HashMap<RepoId, Database>>> = RwLock::new(HashMap::new());
        let peer_id = PeerId::new("test_peer");
        let repo_id = Uuid::new_v4();
        
        // Ensure shadow DB exists
        ensure_shadow_db(&remotes_dir, &shadow_dbs, &peer_id, &repo_id)?;
        
        // Write some test data (simulating append_remote_op)
        {
            let dbs = shadow_dbs.read().unwrap();
            let peer_repos = dbs.get(&peer_id).unwrap();
            let db = peer_repos.get(&repo_id).unwrap();
            
            let doc_id = DocId::new();
            let entry = LedgerEntry {
                doc_id,
                op: Op::Insert { pos: 0, content: "test".to_string() },
                timestamp: 1000,
            };
            
            // Direct write for testing
            let write_txn = db.begin_write()?;
            {
                let mut table = write_txn.open_table(LEDGER_OPS)?;
                let json = serde_json::to_vec(&entry)?;
                table.insert(1u64, json.as_slice())?;
                
                let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
                doc_ops.insert(doc_id.as_u128(), 1u64)?;
            }
            write_txn.commit()?;
        }
        
        // Create read-only ShadowRepo view
        let dbs = shadow_dbs.read().unwrap();
        let peer_repos = dbs.get(&peer_id).unwrap();
        let db = peer_repos.get(&repo_id).unwrap();
        let shadow_repo = ShadowRepo::new(peer_id.clone(), repo_id, db);
        
        // Verify read-only access works
        let max_seq = shadow_repo.get_global_max_seq()?;
        assert_eq!(max_seq, 1);
        
        Ok(())
    }
}
