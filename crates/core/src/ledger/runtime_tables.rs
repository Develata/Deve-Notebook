//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/authority#facts-partition
//!   - 05_diff_logic#source-control-runtime
//!
use crate::ledger::schema::{CLIENT_OP_INDEX, LEDGER_OPS};
use crate::models::deserialize_ledger_entry;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable, TableError, TableHandle};
use std::collections::BTreeMap;

pub(crate) fn repair_client_op_index(db: &Database) -> Result<bool> {
    let write_txn = db.begin_write()?;
    require_write_table(
        &write_txn,
        LEDGER_OPS.name(),
        "Broken client op index rebuild: ledger_ops authority table missing",
    )?;
    let table_missing = !write_table_exists(&write_txn, CLIENT_OP_INDEX.name())?;
    let expected = collect_client_op_entries(&write_txn)?;
    let mut changed = table_missing;
    {
        let mut client_ops = write_txn.open_table(CLIENT_OP_INDEX)?;
        let mut existing = BTreeMap::new();
        for item in client_ops.iter()? {
            let (key, seq) = item?;
            existing.insert(key.value(), seq.value());
        }
        if existing != expected {
            for key in existing.keys() {
                if !expected.contains_key(key) {
                    client_ops.remove(*key)?;
                }
            }
            for (key, seq) in expected {
                client_ops.insert(key, seq)?;
            }
            changed = true;
        }
    }
    if !changed {
        return Ok(false);
    }
    write_txn.commit()?;
    Ok(true)
}

pub(crate) fn repair_client_op_index_if_missing(db: &Database) -> Result<bool> {
    let read_txn = db.begin_read()?;
    match read_txn.open_table(CLIENT_OP_INDEX) {
        Ok(_) => Ok(false),
        Err(TableError::TableDoesNotExist(_)) => {
            drop(read_txn);
            repair_client_op_index(db)
        }
        Err(err) => Err(err.into()),
    }
}

fn collect_client_op_entries(
    write_txn: &redb::WriteTransaction,
) -> Result<BTreeMap<(u64, u64), u64>> {
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut entries = BTreeMap::new();

    for item in ops.iter()? {
        let (seq, bytes) = item?;
        let entry = deserialize_ledger_entry(bytes.value())?;
        let seq = seq.value();
        let (doc_id, client_id, client_op_id) =
            match (entry.doc_id, entry.client_id, entry.client_op_id) {
                (Some(doc_id), Some(client_id), Some(client_op_id)) => {
                    (doc_id, client_id, client_op_id)
                }
                (Some(_), None, None) | (None, None, None) => continue,
                _ => {
                    return Err(anyhow!(
                        "Broken client op index rebuild: incomplete client op metadata at seq {}",
                        seq
                    ));
                }
            };
        let key = (client_id, client_op_id);
        if let Some(previous_seq) = entries.get(&key) {
            tracing::warn!(
                client_id,
                client_op_id,
                first_seq = *previous_seq,
                duplicate_seq = seq,
                %doc_id,
                "Ignoring duplicate client op metadata while rebuilding client_op_index"
            );
            continue;
        }
        entries.insert(key, seq);
    }

    Ok(entries)
}

fn require_write_table(
    write_txn: &redb::WriteTransaction,
    name: &str,
    missing_message: &'static str,
) -> Result<()> {
    if write_table_exists(write_txn, name)? {
        Ok(())
    } else {
        Err(anyhow!(missing_message))
    }
}

fn write_table_exists(write_txn: &redb::WriteTransaction, name: &str) -> Result<bool> {
    for table in write_txn.list_tables()? {
        if table.name() == name {
            return Ok(true);
        }
    }
    Ok(false)
}
