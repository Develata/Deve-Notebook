#![allow(dead_code)]

use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::LedgerEntry;
use redb::ReadableTable;
use std::path::Path;

pub fn create_initialized_local_repo(ledger_dir: &Path, name: &str, url: &str) -> RepoInfo {
    create_initialized_local_repo_with_depth(ledger_dir, 8, name, url)
}

pub fn create_initialized_local_repo_with_depth(
    ledger_dir: &Path,
    snapshot_depth: usize,
    name: &str,
    url: &str,
) -> RepoInfo {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, Some(name), Some(url))
        .expect("initialized local repo");
    repo.get_repo_info()
        .expect("local repo info")
        .expect("local repo metadata")
}

pub fn append_unvalidated_local_op(
    repo: &RepoManager,
    repo_name: &str,
    entry: &LedgerEntry,
) -> u64 {
    repo.run_on_local_repo(repo_name, |db| {
        let write = db.begin_write()?;
        let seq = {
            let mut ops = write.open_table(LEDGER_OPS)?;
            let mut doc_ops = write.open_multimap_table(DOC_OPS)?;
            let mut node_ops = write.open_multimap_table(NODE_OPS)?;
            let mut peer_seqs = write.open_table(PEER_DOC_SEQ)?;
            let next_seq = ops.last()?.map(|(key, _)| key.value() + 1).unwrap_or(1);
            let bytes = bincode::serialize(entry)?;
            ops.insert(next_seq, bytes.as_slice())?;
            if let Some(doc_id) = entry.doc_id {
                doc_ops.insert(doc_id.as_u128(), next_seq)?;
                let peer_key = (doc_id.as_u128(), entry.peer_id.as_str());
                let current = peer_seqs
                    .get(peer_key)?
                    .map(|value| value.value())
                    .unwrap_or(0);
                if entry.seq > current {
                    peer_seqs.insert(peer_key, entry.seq)?;
                }
            }
            if let Some(node_id) = entry.structure_node_id() {
                node_ops.insert(node_id.as_u128(), next_seq)?;
            }
            next_seq
        };
        write.commit()?;
        Ok(seq)
    })
    .expect("append unvalidated local op")
}
