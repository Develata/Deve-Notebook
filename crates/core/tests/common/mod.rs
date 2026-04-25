#![allow(dead_code)]

use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ, REPO_METADATA};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::LedgerEntry;
use redb::ReadableTable;
use std::path::{Path, PathBuf};

pub fn create_initialized_local_repo(ledger_dir: &Path, name: &str, url: &str) -> RepoInfo {
    create_initialized_local_repo_with_depth(ledger_dir, 8, name, url)
}

pub fn create_initialized_local_repo_with_depth(
    ledger_dir: &Path,
    snapshot_depth: usize,
    name: &str,
    url: &str,
) -> RepoInfo {
    try_create_initialized_local_repo_with_depth(ledger_dir, snapshot_depth, name, url)
        .expect("initialized local repo")
}

pub fn try_create_initialized_local_repo_with_depth(
    ledger_dir: &Path,
    snapshot_depth: usize,
    name: &str,
    url: &str,
) -> anyhow::Result<RepoInfo> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, Some(name), Some(url))?;
    repo.get_repo_info()?
        .ok_or_else(|| anyhow::anyhow!("local repo metadata missing for {name}"))
}

pub fn write_repo_metadata(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit metadata");
}

pub fn local_repo_file(ledger_dir: &Path, stem: &str) -> PathBuf {
    ledger_dir.join("local").join(format!("{stem}.redb"))
}

pub fn seed_broken_local_repo_file(ledger_dir: &Path, stem: &str) {
    std::fs::write(local_repo_file(ledger_dir, stem), b"not-a-redb")
        .expect("broken local repo file");
}

pub fn seed_metadata_less_local_repo(ledger_dir: &Path, stem: &str) {
    std::fs::create_dir_all(ledger_dir.join("local")).expect("create local dir");
    let db =
        redb::Database::create(local_repo_file(ledger_dir, stem)).expect("metadata-less repo db");
    db.begin_write()
        .expect("write txn")
        .commit()
        .expect("commit metadata-less db");
    drop(db);
}

#[cfg(unix)]
pub fn seed_invalid_stem_local_repo(ledger_dir: &Path) {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let invalid_path = ledger_dir
        .join("local")
        .join(OsString::from_vec(vec![0xff, b'.', b'r', b'e', b'd', b'b']));
    let invalid = redb::Database::create(&invalid_path).expect("invalid stem db");
    invalid
        .begin_write()
        .expect("write txn")
        .commit()
        .expect("commit invalid stem db");
    drop(invalid);
}

pub fn seed_broken_remote_shadow_repo(ledger_dir: &Path, peer: &str, stem: &str) {
    let remote_dir = ledger_dir.join("remotes").join(peer);
    std::fs::create_dir_all(&remote_dir).expect("remote dir");
    std::fs::write(remote_dir.join(format!("{stem}.redb")), b"not-a-redb")
        .expect("broken shadow repo file");
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
