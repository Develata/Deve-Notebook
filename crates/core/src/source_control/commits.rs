// crates\core\src\source_control
//! # 提交管理 (Commits Manager)
//!
//! 管理提交历史，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `commits` - 存储提交元数据 (序列化 JSON)
//! - Table: `commits_order` - 存储提交顺序索引

use crate::source_control::CommitInfo;
use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};

/// 提交表定义 (commit_id -> JSON)
pub const COMMITS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("commits");
/// 提交顺序表 (序号 -> commit_id)
pub const COMMITS_ORDER_TABLE: TableDefinition<u64, &str> = TableDefinition::new("commits_order");

/// 初始化提交表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(COMMITS_TABLE)?;
        let _ = write_txn.open_table(COMMITS_ORDER_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 创建新提交
pub fn create(db: &Database, message: &str, doc_count: u32, ledger_seq: u64) -> Result<CommitInfo> {
    let commit_id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp_millis();

    let info = CommitInfo {
        id: commit_id.clone(),
        message: message.to_string(),
        timestamp,
        doc_count,
        ledger_seq,
    };

    let json = serde_json::to_string(&info)?;

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(COMMITS_TABLE)?;
        table.insert(commit_id.as_str(), json.as_str())?;

        let mut order_table = write_txn.open_table(COMMITS_ORDER_TABLE)?;
        let next_seq = next_seq_inner(&order_table)?;
        order_table.insert(next_seq, commit_id.as_str())?;
    }
    write_txn.commit()?;

    tracing::info!("Created commit: {} - {}", commit_id, message);
    Ok(info)
}

/// 获取下一个序列号
fn next_seq_inner(table: &redb::Table<u64, &str>) -> Result<u64> {
    let mut max_seq = 0u64;
    for entry in table.iter()? {
        let (seq, _) = entry?;
        if seq.value() > max_seq {
            max_seq = seq.value();
        }
    }
    Ok(max_seq + 1)
}

/// 获取提交历史 (最新的在前)
pub fn list(db: &Database, limit: u32) -> Result<Vec<CommitInfo>> {
    let read_txn = db.begin_read()?;
    let order_table = read_txn.open_table(COMMITS_ORDER_TABLE)?;
    let commits_table = read_txn.open_table(COMMITS_TABLE)?;

    // 收集所有序号并降序排列
    let mut seqs: Vec<u64> = order_table
        .iter()?
        .filter_map(|e| e.ok().map(|(k, _)| k.value()))
        .collect();
    seqs.sort_by(|a, b| b.cmp(a));
    seqs.truncate(limit as usize);

    let mut commits = Vec::new();
    for seq in seqs {
        if let Some(commit_id) = order_table.get(seq)?
            && let Some(json) = commits_table.get(commit_id.value())?
                && let Ok(info) = serde_json::from_str::<CommitInfo>(json.value()) {
                    commits.push(info);
                }
    }

    Ok(commits)
}
