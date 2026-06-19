use super::AppState;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, PEER_DOC_SEQ};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, serialize_ledger_entry};
use redb::ReadableTable;
use std::sync::Arc;

pub(super) fn inject_legacy_invalid_insert(
    state: &Arc<AppState>,
    doc_id: DocId,
    peer_id: PeerId,
) -> anyhow::Result<()> {
    state.repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        let next_global_seq = write
            .open_table(LEDGER_OPS)?
            .last()?
            .map(|(seq, _)| seq.value() + 1)
            .unwrap_or(1);
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 4,
                content: "!".into(),
            },
            2,
            peer_id.clone(),
            2,
            None,
            None,
        );
        let bytes = serialize_ledger_entry(&entry)?;
        write
            .open_table(LEDGER_OPS)?
            .insert(next_global_seq, bytes.as_slice())?;
        write
            .open_multimap_table(DOC_OPS)?
            .insert(doc_id.as_u128(), next_global_seq)?;
        write
            .open_table(PEER_DOC_SEQ)?
            .insert((doc_id.as_u128(), peer_id.as_str()), 2)?;
        write.commit()?;
        Ok(())
    })
}
