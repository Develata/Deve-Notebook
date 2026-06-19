//! plan_ref: infra
//!
use std::sync::{LazyLock, Mutex, MutexGuard};

use crate::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    RepoInfo, RepoManager,
};
use std::path::Path;

static LOCAL_REPO_CATALOG_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn local_repo_catalog_test_guard() -> MutexGuard<'static, ()> {
    LOCAL_REPO_CATALOG_TEST_MUTEX
        .lock()
        .expect("local repo catalog test mutex")
}

pub(crate) fn create_initialized_local_repo(ledger_dir: &Path, name: &str, url: &str) -> RepoInfo {
    create_initialized_local_repo_with_depth(ledger_dir, 8, name, url)
}

pub(crate) fn create_initialized_local_repo_with_depth(
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

pub(crate) fn write_repo_metadata(db: &redb::Database, info: &RepoInfo) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = bincode::serialize(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let bytes = bincode::serialize(info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

pub(crate) fn delete_repo_metadata(db: &redb::Database) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    txn.delete_table(REPO_METADATA)?;
    txn.commit()?;
    Ok(())
}

pub(crate) fn poison_repo_metadata_invalid_bincode(db: &redb::Database) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = bincode::serialize(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        table.insert(&REPO_INFO_METADATA_KEY, b"not-bincode".as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

pub(crate) fn create_repo_db_missing_metadata(path: impl AsRef<Path>) {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent).expect("metadata-less repo parent dir");
    }
    let db = redb::Database::create(path.as_ref()).expect("metadata-less repo db");
    db.begin_write()
        .expect("write txn")
        .commit()
        .expect("commit metadata-less db");
    drop(db);
}

pub(crate) fn seed_shadow_repo_missing_metadata(repo: &RepoManager, peer_name: &str, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_name);
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    create_repo_db_missing_metadata(peer_dir.join(format!("{stem}.redb")));
}
