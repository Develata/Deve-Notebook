// crates\core\src\source_control
//! # 提交管理 (Commits Manager)
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 07_diff_logic#source-control-runtime
//!
//! 管理提交历史，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `commits` - 存储提交元数据 (序列化 JSON)
//! - Table: `commits_order` - 存储提交顺序索引

use crate::source_control::CommitInfo;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable, TableDefinition};

#[path = "commits_repair.rs"]
mod repair;
pub use self::repair::repair_missing_order_table;

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
///
/// **Invariant**: parent_id 自动从最新提交中推导，构成单链提交历史。
pub fn create(db: &Database, message: &str, doc_count: u32, ledger_seq: u64) -> Result<CommitInfo> {
    let commit_id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp_millis();
    let parent_id = get_latest_id(db)?;

    let info = CommitInfo {
        id: commit_id.clone(),
        parent_id,
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

    tracing::info!(
        "Created commit: {} (parent: {:?}) - {}",
        commit_id,
        info.parent_id,
        message
    );
    Ok(info)
}

/// 获取下一个序列号 — O(log n) via `last()`
fn next_seq_inner(table: &redb::Table<u64, &str>) -> Result<u64> {
    let last_seq = table.last()?.map(|(k, _)| k.value()).unwrap_or(0);
    Ok(last_seq + 1)
}

/// 获取最新提交的 ID（作为新提交的 parent_id）— O(log n) via `last()`
pub fn get_latest_id(db: &Database) -> Result<Option<String>> {
    let read_txn = db.begin_read()?;
    let order_table = read_txn
        .open_table(COMMITS_ORDER_TABLE)
        .map_err(|err| broken_commit_history("opening commit order table", err))?;
    match order_table.last()? {
        Some((_, commit_id)) => Ok(Some(commit_id.value().to_string())),
        None => Ok(None),
    }
}

/// 获取提交历史 (最新的在前)
pub fn list(db: &Database, limit: u32) -> Result<Vec<CommitInfo>> {
    let read_txn = db.begin_read()?;
    let order_table = read_txn
        .open_table(COMMITS_ORDER_TABLE)
        .map_err(|err| broken_commit_history("opening commit order table", err))?;
    let commits_table = read_txn
        .open_table(COMMITS_TABLE)
        .map_err(|err| broken_commit_history("opening commit payload table", err))?;

    // 收集所有序号并降序排列
    let mut seqs = Vec::new();
    for entry in order_table.iter()? {
        let (key, _) = entry?;
        seqs.push(key.value());
    }
    seqs.sort_by(|a, b| b.cmp(a));
    seqs.truncate(limit as usize);

    let mut commits = Vec::new();
    for seq in seqs {
        let commit_id = order_table.get(seq)?.ok_or_else(|| {
            anyhow!(
                "Broken commit history while reading order row {}: commit id missing",
                seq
            )
        })?;
        let json = commits_table.get(commit_id.value())?.ok_or_else(|| {
            anyhow!(
                "Broken commit history while reading order row {}: missing commit payload for {}",
                seq,
                commit_id.value()
            )
        })?;
        let info = serde_json::from_str::<CommitInfo>(json.value())?;
        commits.push(info);
    }

    Ok(commits)
}

fn broken_commit_history(action: &str, err: redb::TableError) -> anyhow::Error {
    anyhow!("Broken commit history while {action}: {err}")
}

#[cfg(test)]
mod tests {
    use super::{COMMITS_ORDER_TABLE, COMMITS_TABLE, create, get_latest_id, list};
    use tempfile::TempDir;

    #[test]
    fn commit_list_fails_closed_on_broken_commit_payload() {
        let dir = TempDir::new().expect("tempdir");
        let db = redb::Database::create(dir.path().join("commits.redb")).expect("db");
        super::init_table(&db).expect("init tables");
        let info = create(&db, "initial", 1, 1).expect("create commit");

        let txn = db.begin_write().expect("write txn");
        txn.open_table(COMMITS_TABLE)
            .expect("commits table")
            .insert(info.id.as_str(), "not-json")
            .expect("poison commit");
        txn.commit().expect("commit poison");

        let err = list(&db, 10).expect_err("broken commit payload must fail closed");
        assert!(
            err.to_string().contains("expected ident")
                || err.to_string().contains("expected value")
                || err.to_string().contains("EOF")
        );
    }

    #[test]
    fn commit_list_returns_latest_first() {
        let dir = TempDir::new().expect("tempdir");
        let db = redb::Database::create(dir.path().join("commits.redb")).expect("db");
        super::init_table(&db).expect("init tables");
        let first = create(&db, "first", 1, 1).expect("first");
        let second = create(&db, "second", 1, 2).expect("second");

        let listed = list(&db, 10).expect("list commits");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
        let txn = db.begin_read().expect("read txn");
        let order = txn.open_table(COMMITS_ORDER_TABLE).expect("order table");
        assert!(order.get(1).expect("order first").is_some());
        assert!(order.get(2).expect("order second").is_some());
    }

    #[test]
    fn commit_list_fails_closed_when_tables_are_missing() {
        let dir = TempDir::new().expect("tempdir");
        let db = redb::Database::create(dir.path().join("commits.redb")).expect("db");

        let err = list(&db, 10).expect_err("missing commit tables must fail closed");
        assert!(err.to_string().contains("Broken commit history"));

        let err = get_latest_id(&db).expect_err("missing order table must fail closed");
        assert!(err.to_string().contains("Broken commit history"));
    }

    #[test]
    fn commit_list_fails_closed_when_order_row_points_to_missing_payload() {
        let dir = TempDir::new().expect("tempdir");
        let db = redb::Database::create(dir.path().join("commits.redb")).expect("db");
        super::init_table(&db).expect("init tables");
        let info = create(&db, "initial", 1, 1).expect("create commit");

        let txn = db.begin_write().expect("write txn");
        txn.open_table(COMMITS_TABLE)
            .expect("commits table")
            .remove(info.id.as_str())
            .expect("remove commit payload");
        txn.commit().expect("commit poison");

        let err = list(&db, 10).expect_err("missing commit payload must fail closed");
        assert!(err.to_string().contains("Broken commit history"));
    }
}
