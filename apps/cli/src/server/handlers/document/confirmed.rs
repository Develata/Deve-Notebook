use deve_core::models::{DocId, LedgerEntry};
use deve_core::protocol::{ClientOrigin, ConfirmedOp};

pub(super) fn load_doc_ops(db: &redb::Database, doc_id: DocId) -> anyhow::Result<Vec<ConfirmedOp>> {
    let entries = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
    Ok(project_entries(entries))
}

pub(super) fn load_doc_ops_after(
    db: &redb::Database,
    doc_id: DocId,
    min_seq: u64,
) -> anyhow::Result<Vec<ConfirmedOp>> {
    let entries = deve_core::ledger::ops::get_ops_from_db_after(db, doc_id, min_seq)?;
    Ok(project_entries(entries))
}

fn project_entries(entries: Vec<(u64, LedgerEntry)>) -> Vec<ConfirmedOp> {
    entries
        .into_iter()
        .map(|(seq, entry)| {
            let origin = origin_of(&entry);
            ConfirmedOp::new(seq, entry.op, origin)
        })
        .collect()
}

fn origin_of(entry: &LedgerEntry) -> Option<ClientOrigin> {
    Some(ClientOrigin {
        client_id: entry.client_id?,
        client_op_id: entry.client_op_id?,
    })
}
