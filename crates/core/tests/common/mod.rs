#![allow(dead_code)]

use deve_core::codec;
use deve_core::ledger::schema::{
    CLIENT_OP_INDEX, DOC_OPS, DOCID_TO_PATH, INODE_TO_DOCID, INODE_TO_NODEID, LEDGER_OPS, NODE_OPS,
    NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID, PEER_FACT_OPS, PEER_FACT_SEQ,
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    SNAPSHOT_DATA, SNAPSHOT_INDEX,
};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{LedgerEntry, PeerId, serialize_ledger_entry};
use redb::ReadableTable;
use std::path::{Path, PathBuf};

const REMOTE_IMPORT_SESSIONS: redb::TableDefinition<u128, &[u8]> =
    redb::TableDefinition::new("remote_import_sessions");
const REMOTE_IMPORT_RUNTIME: redb::TableDefinition<u8, &[u8]> =
    redb::TableDefinition::new("remote_import_runtime");
const PROJECTION_FAULTS: redb::TableDefinition<[u8; 32], &[u8]> =
    redb::TableDefinition::new("projection_faults");

/// Full production creation choreography for a catalog-backed local repo,
/// mirroring `deve_core`'s internal `test_support::init_cataloged_repo` via the
/// public API: UUID-canonical machine name, prepared locator + workspace
/// identity marker, and a committed `Normal` catalog membership record. Bare
/// `RepoManager::init` repos are invisible to catalog-backed resolution/listing.
pub fn init_cataloged_repo(
    ledger_dir: &Path,
    projection_base: &Path,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    init_cataloged_repo_with(ledger_dir, projection_base, 8, uuid::Uuid::new_v4(), None)
}

/// Variant of [`init_cataloged_repo`] that records a repo URL in metadata.
pub fn init_cataloged_repo_with_url(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_url: &str,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    init_cataloged_repo_with(
        ledger_dir,
        projection_base,
        8,
        uuid::Uuid::new_v4(),
        Some(repo_url),
    )
}

/// Variant of [`init_cataloged_repo`] that preserves a specific snapshot depth.
pub fn init_cataloged_repo_with_depth(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    init_cataloged_repo_with(
        ledger_dir,
        projection_base,
        snapshot_depth,
        uuid::Uuid::new_v4(),
        None,
    )
}

/// Variant of [`init_cataloged_repo`] that binds an explicit `repo_id`, so
/// separate ledgers (e.g. a sync source and receiver) can catalog repos that
/// share the same RepoId.
pub fn init_cataloged_repo_with_id(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_id: uuid::Uuid,
    repo_url: &str,
) -> anyhow::Result<RepoManager> {
    let (repo, _repo_id) =
        init_cataloged_repo_with(ledger_dir, projection_base, 8, repo_id, Some(repo_url))?;
    Ok(repo)
}

fn init_cataloged_repo_with(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_id: uuid::Uuid,
    repo_url: Option<&str>,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    use deve_core::ledger::init::RepoInitOptions;

    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        ledger_dir,
        snapshot_depth,
        Some(&execution_name),
        RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: repo_url.map(str::to_string),
        },
    )?;
    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, projection_base)?;
    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    deve_core::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    Ok((repo, repo_id))
}

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

pub fn read_repo_metadata(db: &redb::Database) -> RepoInfo {
    let txn = db.begin_read().expect("read txn");
    let table = txn.open_table(REPO_METADATA).expect("repo metadata");
    let bytes = table
        .get(&REPO_INFO_METADATA_KEY)
        .expect("read repo metadata")
        .expect("repo metadata row");
    codec::decode(bytes.value()).expect("decode repo metadata")
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
        let _ = txn
            .open_table(REMOTE_IMPORT_SESSIONS)
            .expect("remote import sessions");
        let _ = txn
            .open_table(REMOTE_IMPORT_RUNTIME)
            .expect("remote import runtime");
        let _ = txn
            .open_table(PROJECTION_FAULTS)
            .expect("projection faults");
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
    let _ = txn.open_table(PEER_FACT_SEQ).expect("peer_fact_seq");
    let _ = txn.open_table(PEER_FACT_OPS).expect("peer_fact_ops");
    let _ = txn
        .open_multimap_table(SNAPSHOT_INDEX)
        .expect("snapshot_index");
    let _ = txn.open_table(SNAPSHOT_DATA).expect("snapshot_data");
    let _ = txn
        .open_table(REMOTE_IMPORT_SESSIONS)
        .expect("remote import sessions");
    let _ = txn
        .open_table(REMOTE_IMPORT_RUNTIME)
        .expect("remote import runtime");
    let _ = txn
        .open_table(PROJECTION_FAULTS)
        .expect("projection faults");
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
            let mut peer_seqs = write.open_table(PEER_FACT_SEQ)?;
            let mut peer_ops = write.open_table(PEER_FACT_OPS)?;
            let next_seq = ops.last()?.map(|(key, _)| key.value() + 1).unwrap_or(1);
            let bytes = serialize_ledger_entry(entry)?;
            ops.insert(next_seq, bytes.as_slice())?;
            if let Some(doc_id) = entry.doc_id {
                doc_ops.insert(doc_id.as_u128(), next_seq)?;
                let peer_key = entry.origin_peer_id.as_str();
                let current = peer_seqs
                    .get(peer_key)?
                    .map(|value| value.value())
                    .unwrap_or(0);
                if entry.peer_seq.get() > current {
                    peer_seqs.insert(peer_key, entry.peer_seq.get())?;
                }
            }
            if let Some(node_id) = entry.structure_node_id() {
                node_ops.insert(node_id.as_u128(), next_seq)?;
            }
            peer_ops.insert(
                (entry.origin_peer_id.as_str(), entry.peer_seq.get()),
                next_seq,
            )?;
            next_seq
        };
        write.commit()?;
        Ok(seq)
    })
    .expect("append unvalidated local op")
}
