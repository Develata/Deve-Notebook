//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 07_diff_logic#source-control-runtime

use anyhow::Result;
use redb::{Database, ReadableTable, TableError};

use super::{COMMITS_ORDER_TABLE, COMMITS_TABLE, CommitInfo};

pub fn repair_missing_order_table(db: &Database) -> Result<()> {
    let read_txn = db.begin_read()?;
    match read_txn.open_table(COMMITS_ORDER_TABLE) {
        Ok(_) => return Ok(()),
        Err(TableError::TableDoesNotExist(_)) => {}
        Err(err) => return Err(err.into()),
    }
    drop(read_txn);

    let write_txn = db.begin_write()?;
    {
        let commits_table = write_txn.open_table(COMMITS_TABLE)?;
        let mut order_table = write_txn.open_table(COMMITS_ORDER_TABLE)?;
        if order_table.last()?.is_none() {
            let mut commits = Vec::new();
            for entry in commits_table.iter()? {
                let (_, json) = entry?;
                commits.push(serde_json::from_str::<CommitInfo>(json.value())?);
            }
            commits.sort_by(|left, right| {
                left.ledger_seq
                    .cmp(&right.ledger_seq)
                    .then(left.timestamp.cmp(&right.timestamp))
                    .then(left.id.cmp(&right.id))
            });
            for (idx, commit) in commits.iter().enumerate() {
                order_table.insert((idx + 1) as u64, commit.id.as_str())?;
            }
        }
    }
    write_txn.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::repair_missing_order_table;
    use crate::source_control::commits::{COMMITS_ORDER_TABLE, create, list};
    use tempfile::TempDir;

    #[test]
    fn repair_rebuilds_missing_order_table_from_commit_payloads() {
        let dir = TempDir::new().expect("tempdir");
        let db = redb::Database::create(dir.path().join("commits.redb")).expect("db");
        super::super::init_table(&db).expect("init tables");
        let first = create(&db, "first", 1, 1).expect("first");
        let second = create(&db, "second", 1, 2).expect("second");

        let write_txn = db.begin_write().expect("write txn");
        let _ = write_txn
            .delete_table(COMMITS_ORDER_TABLE)
            .expect("delete order table");
        write_txn.commit().expect("commit delete");

        repair_missing_order_table(&db).expect("repair order table");

        let listed = list(&db, 10).expect("list commits");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
    }
}
