#![allow(dead_code)]

use deve_core::codec;
use deve_core::ledger::schema::{
    CLIENT_OP_INDEX, DOC_OPS, DOCID_TO_PATH, INODE_TO_DOCID, INODE_TO_NODEID, LEDGER_OPS, NODE_OPS,
    NODE_PEER_SEQ, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID, PEER_DOC_SEQ,
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    SNAPSHOT_DATA, SNAPSHOT_INDEX,
};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{LedgerEntry, PeerId, serialize_ledger_entry};
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
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&REDB_SCHEMA_VERSION)
                    .expect("encode schema version")
                    .as_slice(),
            )
            .expect("write schema version");
        table
            .insert(
                &REPO_INFO_METADATA_KEY,
                codec::encode(info).expect("encode metadata").as_slice(),
            )
            .expect("write metadata");
    }
    txn.commit().expect("commit metadata");
}

pub fn delete_repo_metadata(db: &redb::Database) {
    let txn = db.begin_write().expect("write txn");
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .remove(&REPO_INFO_METADATA_KEY)
            .expect("delete repo info metadata");
    }
    txn.commit().expect("commit missing metadata");
}

pub fn poison_repo_metadata_invalid_codec(db: &redb::Database) {
    let txn = db.begin_write().expect("write txn");
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&REDB_SCHEMA_VERSION)
                    .expect("encode schema version")
                    .as_slice(),
            )
            .expect("write schema version");
        table
            .insert(&REPO_INFO_METADATA_KEY, [0_u8, 1, 2, 3].as_slice())
            .expect("write broken metadata");
    }
    txn.commit().expect("commit broken metadata");
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
    let txn = db.begin_write().expect("write txn");
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&REDB_SCHEMA_VERSION)
                    .expect("encode schema version")
                    .as_slice(),
            )
            .expect("write schema version");
    }
    txn.commit().expect("commit metadata-less db");
    drop(db);
}

pub fn seed_non_file_local_repo_entry(ledger_dir: &Path, stem: &str) {
    std::fs::create_dir_all(local_repo_file(ledger_dir, stem)).expect("non-file local repo entry");
}

pub fn seed_local_repo_missing_source_control_tables(path: &Path, info: &RepoInfo) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("legacy local repo parent");
    }
    let db = redb::Database::create(path).expect("create legacy local db");
    init_core_repo_tables(&db);
    write_repo_metadata(&db, info);
}

fn init_core_repo_tables(db: &redb::Database) {
    let txn = db.begin_write().expect("write txn");
    let _ = txn.open_table(DOCID_TO_PATH).expect("docid_to_path");
    let _ = txn.open_table(PATH_TO_DOCID).expect("path_to_docid");
    let _ = txn.open_table(INODE_TO_DOCID).expect("inode_to_docid");
    let _ = txn.open_table(NODEID_TO_META).expect("nodeid_to_meta");
    let _ = txn.open_table(PATH_TO_NODEID).expect("path_to_nodeid");
    let _ = txn.open_table(INODE_TO_NODEID).expect("inode_to_nodeid");
    let _ = txn.open_table(LEDGER_OPS).expect("ledger_ops");
    let _ = txn.open_multimap_table(DOC_OPS).expect("doc_ops");
    let _ = txn.open_multimap_table(NODE_OPS).expect("node_ops");
    let _ = txn.open_table(CLIENT_OP_INDEX).expect("client_op_index");
    let _ = txn.open_table(NODE_PEER_SEQ).expect("node_peer_seq");
    let _ = txn
        .open_multimap_table(SNAPSHOT_INDEX)
        .expect("snapshot_index");
    let _ = txn.open_table(SNAPSHOT_DATA).expect("snapshot_data");
    txn.commit().expect("commit core repo tables");
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

pub fn seed_broken_remote_shadow_repo(ledger_dir: &Path, peer_id: &PeerId, stem: &str) {
    let remote_dir = ledger_dir.join("remotes").join(peer_id.to_filename());
    std::fs::create_dir_all(&remote_dir).expect("remote dir");
    std::fs::write(remote_dir.join(format!("{stem}.redb")), b"not-a-redb")
        .expect("broken shadow repo file");
}

pub fn seed_shadow_repo_info(repo: &RepoManager, peer_id: &PeerId, stem: &str, info: &RepoInfo) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let db = redb::Database::create(peer_dir.join(format!("{stem}.redb"))).expect("shadow repo db");
    write_repo_metadata(&db, info);
}

pub fn seed_metadata_less_shadow_repo(repo: &RepoManager, peer_id: &PeerId, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let db = redb::Database::create(peer_dir.join(format!("{stem}.redb")))
        .expect("metadata-less shadow repo");
    let txn = db.begin_write().expect("write txn");
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&REDB_SCHEMA_VERSION)
                    .expect("encode schema version")
                    .as_slice(),
            )
            .expect("write schema version");
    }
    txn.commit().expect("commit metadata-less shadow repo");
    drop(db);
}

pub fn seed_non_file_shadow_repo_entry(repo: &RepoManager, peer_id: &PeerId, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(peer_dir.join(format!("{stem}.redb")))
        .expect("non-file shadow repo entry");
}

pub fn seed_shadow_without_metadata_row(repo: &RepoManager, peer_id: &PeerId, repo_id: uuid::Uuid) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let db = redb::Database::create(peer_dir.join(format!("{repo_id}.redb")))
        .expect("legacy shadow repo");
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &REPO_SCHEMA_VERSION_METADATA_KEY,
            codec::encode(&REDB_SCHEMA_VERSION)
                .expect("encode schema version")
                .as_slice(),
        )
        .expect("write schema version");
    txn.commit().expect("commit legacy shadow");
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
            let bytes = serialize_ledger_entry(entry)?;
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
