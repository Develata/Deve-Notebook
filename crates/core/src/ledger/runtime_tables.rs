//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 04_storage#facts-partition
//!   - 07_diff_logic#source-control-runtime
//!
use crate::ledger::schema::{CLIENT_OP_INDEX, LEDGER_OPS};
use crate::models::deserialize_ledger_entry;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableError};
use std::collections::BTreeMap;

pub(crate) fn repair_missing_client_op_index(db: &Database) -> Result<bool> {
    let read_txn = db.begin_read()?;
    let mut table_missing = false;
    match read_txn.open_table(CLIENT_OP_INDEX) {
        Ok(table) if !table.is_empty()? => return Ok(false),
        Ok(_) => {}
        Err(TableError::TableDoesNotExist(_)) => table_missing = true,
        Err(err) => return Err(err.into()),
    }
    drop(read_txn);

    let entries = collect_client_op_entries(db)?;
    if !table_missing && entries.is_empty() {
        return Ok(false);
    }
    let write_txn = db.begin_write()?;
    {
        let mut client_ops = write_txn.open_table(CLIENT_OP_INDEX)?;
        for (key, seq) in entries {
            client_ops.insert(key, seq)?;
        }
    }
    write_txn.commit()?;
    Ok(true)
}

fn collect_client_op_entries(db: &Database) -> Result<BTreeMap<(u64, u64), u64>> {
    let read_txn = db.begin_read()?;
    let ops = read_txn.open_table(LEDGER_OPS)?;
    let mut entries = BTreeMap::new();

    for item in ops.iter()? {
        let (seq, bytes) = item?;
        let entry = deserialize_ledger_entry(bytes.value())?;
        let (Some(doc_id), Some(client_id), Some(client_op_id)) =
            (entry.doc_id, entry.client_id, entry.client_op_id)
        else {
            continue;
        };
        let key = (client_id, client_op_id);
        let seq = seq.value();
        if let Some(previous_seq) = entries.insert(key, seq) {
            return Err(anyhow!(
                "Broken client op index rebuild: duplicate client op ({}, {}) at seq {} and {} while scanning doc {}",
                client_id,
                client_op_id,
                previous_seq,
                seq,
                doc_id
            ));
        }
    }

    Ok(entries)
}
