//! # 范围查询模块 (Range Query Operations)
//!
//! **架构作用**:
//! 提供基于序列号范围的操作查询功能，用于 P2P 同步的增量数据拉取和推送。
//!
//! **核心功能清单**:
//! - `get_ops_in_range`: 从指定数据库获取范围内的操作。
//! - `get_max_seq`: 获取数据库的最大序列号。
//!
//! **类型**: Core MUST (核心必选)

use anyhow::{Result, Context};
use redb::{Database, ReadableTable};
use crate::models::LedgerEntry;
use crate::ledger::schema::LEDGER_OPS;

/// 从数据库获取指定序列号范围的操作
pub fn get_ops_in_range(db: &Database, start_seq: u64, end_seq: u64) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(LEDGER_OPS)?;
    
    let mut result = Vec::new();
    let range = table.range(start_seq..end_seq)?;
    for item in range {
        let (key, value) = item?;
        let seq = key.value();
        let entry: LedgerEntry = serde_json::from_slice(value.value())
            .with_context(|| format!("Failed to deserialize op at seq {}", seq))?;
        result.push((seq, entry));
    }
    Ok(result)
}

/// 获取数据库的最大序列号
pub fn get_max_seq(db: &Database) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(LEDGER_OPS)?;
    
    let mut max_seq = 0u64;
    for item in table.iter()? {
        let (key, _): (redb::AccessGuard<u64>, _) = item?;
        max_seq = max_seq.max(key.value());
    }
    Ok(max_seq)
}
