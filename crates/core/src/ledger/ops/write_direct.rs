//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract

use super::validate;
use crate::ledger::GlobalSeq;
use crate::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use crate::models::{LedgerEntry, PeerFactSeq, PeerId, serialize_ledger_entry};
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable, WriteTransaction};

pub fn append_op_to_db(db: &Database, entry: &LedgerEntry, repo_scope: &str) -> Result<u64> {
    let write_txn = db.begin_write()?;
    let seq = append_op_to_txn(&write_txn, entry, repo_scope)?;
    write_txn.commit()?;
    Ok(seq)
}

pub(crate) fn append_op_to_txn(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<u64> {
    validate::validate_ledger_append(write_txn, entry, repo_scope)?;
    let expected_peer_seq = next_peer_fact_seq(write_txn, &entry.origin_peer_id)?;
    if entry.peer_seq != expected_peer_seq {
        return Err(anyhow!(
            "PeerFactSeq mismatch for {}: expected {}, received {}",
            entry.origin_peer_id,
            expected_peer_seq,
            entry.peer_seq
        ));
    }
    let seq = {
        let mut ops = write_txn.open_table(LEDGER_OPS)?;
        let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
        let mut node_ops = write_txn.open_multimap_table(NODE_OPS)?;
        let last_seq = ops
            .last()?
            .map(|(k, _)| GlobalSeq::from_storage_key(k.value()))
            .unwrap_or(GlobalSeq::ZERO);
        let new_seq = last_seq
            .next()
            .ok_or_else(|| anyhow::anyhow!("GlobalSeq overflow"))?;
        let new_seq_key = new_seq.storage_key();
        let bytes = serialize_ledger_entry(entry)?;
        ops.insert(new_seq_key, bytes.as_slice())?;
        if let Some(doc_id) = entry.doc_id {
            doc_ops.insert(doc_id.as_u128(), new_seq_key)?;
        }
        if let Some(node_id) = entry.structure_node_id() {
            node_ops.insert(node_id.as_u128(), new_seq_key)?;
        }
        let mut peer_seqs = write_txn.open_table(PEER_FACT_SEQ)?;
        let mut peer_ops = write_txn.open_table(PEER_FACT_OPS)?;
        peer_seqs.insert(entry.origin_peer_id.as_str(), entry.peer_seq.get())?;
        peer_ops.insert(
            (entry.origin_peer_id.as_str(), entry.peer_seq.get()),
            new_seq_key,
        )?;
        new_seq_key
    };
    Ok(seq)
}

pub(crate) fn next_peer_fact_seq(
    write_txn: &WriteTransaction,
    origin_peer_id: &PeerId,
) -> Result<PeerFactSeq> {
    let peer_seqs = write_txn.open_table(PEER_FACT_SEQ)?;
    let current = peer_seqs
        .get(origin_peer_id.as_str())?
        .map(|value| PeerFactSeq::new(value.value()))
        .unwrap_or(PeerFactSeq::ZERO);
    current
        .next()
        .ok_or_else(|| anyhow!("PeerFactSeq overflow for {}", origin_peer_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::RepoManager;
    use crate::models::{DocId, LedgerEntry, Op};

    #[test]
    fn rolled_back_write_transaction_does_not_leave_peer_fact_seq_gap() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let repo = RepoManager::init(dir.path().join("ledger"), 8, None, None)?;
        let repo_name = repo.local_repo_name().to_string();
        let peer = repo.local_peer_id().clone();
        let doc_id = DocId::new();
        repo.run_on_local_repo(&repo_name, |db| {
            let write = db.begin_write()?;
            let entry = LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "rolled back".into(),
                },
                1,
                peer.clone(),
                1,
                None,
                None,
            );
            append_op_to_txn(&write, &entry, "local:test")?;
            drop(write);
            Ok(())
        })?;

        let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
        assert_eq!(repo.get_local_peer_waterline(&repo_id)?, 0_u64);
        let (_global_seq, peer_seq) = repo
            .local_fact_writer(crate::models::FactActor::new("test")?)
            .append_content_in_local_repo(
                &repo_name,
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "committed".into(),
                },
                2,
            )?;
        assert_eq!(peer_seq, 1);
        Ok(())
    }
}
