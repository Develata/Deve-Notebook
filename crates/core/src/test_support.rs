use std::sync::{LazyLock, Mutex, MutexGuard};

use crate::codec;
use crate::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    RepoInfo, RepoManager,
};
use std::path::Path;

static LOCAL_REPO_CATALOG_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
pub(crate) static CWD_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn local_repo_catalog_test_guard() -> MutexGuard<'static, ()> {
    LOCAL_REPO_CATALOG_TEST_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A prepared (not-yet-committed) catalog-backed local repo produced by the
/// shared creation choreography: UUID-canonical machine name, prepared locator,
/// created workspace, workspace identity marker, and process membership seeded
/// from durable records (empty until a membership record is committed).
pub(crate) struct PreparedCatalogRepo {
    pub repo: RepoManager,
    pub repo_id: uuid::Uuid,
    /// `<projection_base>/<workspace_segment>/` created on disk.
    pub workspace: std::path::PathBuf,
}

/// Shared prepare half of the production repo-creation choreography. Callers
/// either commit membership (`commit_cataloged_repo_membership` /
/// `init_cataloged_repo`) or drive the catalog cut themselves. No membership
/// record is committed here, so the repo is not yet visible to catalog-backed
/// resolution/listing.
pub(crate) fn prepare_cataloged_repo(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_url: Option<&str>,
) -> anyhow::Result<PreparedCatalogRepo> {
    let repo_id = uuid::Uuid::new_v4();
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        ledger_dir,
        snapshot_depth,
        Some(&execution_name),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: repo_url.map(str::to_string),
        },
    )?;
    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, projection_base)?;
    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    crate::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
    repo.seed_catalog_membership_from_records()?;
    Ok(PreparedCatalogRepo {
        repo,
        repo_id,
        workspace,
    })
}

/// Commits a `Normal` catalog membership record for a prepared repo, using the
/// production cut authority + permit path.
pub(crate) fn commit_cataloged_repo_membership(
    prepared: &PreparedCatalogRepo,
) -> anyhow::Result<()> {
    let authority = prepared.repo.claim_repo_catalog_cut_authority()?;
    let creation = prepared
        .repo
        .prepare_repo_creation_membership(prepared.repo_id, uuid::Uuid::new_v4())?;
    let revalidated = prepared
        .repo
        .revalidate_repo_creation_membership(&creation)?;
    let permit = authority.permit(prepared.repo_id)?;
    prepared
        .repo
        .commit_repo_creation_membership(&creation, &revalidated, &permit)?;
    Ok(())
}

/// Full production creation choreography for a catalog-backed local repo:
/// UUID-canonical machine name, prepared locator + workspace identity marker,
/// and a committed `Normal` catalog membership record. Bare `RepoManager::init`
/// repos are invisible to catalog-backed resolution and listing.
pub(crate) fn init_cataloged_repo(
    ledger_dir: &Path,
    projection_base: &Path,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let prepared = prepare_cataloged_repo(ledger_dir, projection_base, 8, None)?;
    commit_cataloged_repo_membership(&prepared)?;
    Ok((prepared.repo, prepared.repo_id))
}

/// Variant of [`init_cataloged_repo`] that records a repo URL in metadata.
pub(crate) fn init_cataloged_repo_with_url(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_url: &str,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let prepared = prepare_cataloged_repo(ledger_dir, projection_base, 8, Some(repo_url))?;
    commit_cataloged_repo_membership(&prepared)?;
    Ok((prepared.repo, prepared.repo_id))
}

/// Variant of [`init_cataloged_repo`] that preserves a specific snapshot depth.
pub(crate) fn init_cataloged_repo_with_depth(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let prepared = prepare_cataloged_repo(ledger_dir, projection_base, snapshot_depth, None)?;
    commit_cataloged_repo_membership(&prepared)?;
    Ok((prepared.repo, prepared.repo_id))
}

pub(crate) fn write_repo_metadata(db: &redb::Database, info: &RepoInfo) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let bytes = codec::encode(info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

pub(crate) fn delete_repo_metadata(db: &redb::Database) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        table.remove(&REPO_INFO_METADATA_KEY)?;
    }
    txn.commit()?;
    Ok(())
}

pub(crate) fn poison_repo_metadata_invalid_codec(db: &redb::Database) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        table.insert(&REPO_INFO_METADATA_KEY, b"not-postcard".as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

pub(crate) fn create_repo_db_missing_metadata(path: impl AsRef<Path>) {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent).expect("metadata-less repo parent dir");
    }
    let db = redb::Database::create(path.as_ref()).expect("metadata-less repo db");
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

pub(crate) fn seed_shadow_repo_missing_metadata(repo: &RepoManager, peer_name: &str, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_name);
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    create_repo_db_missing_metadata(peer_dir.join(format!("{stem}.redb")));
}
