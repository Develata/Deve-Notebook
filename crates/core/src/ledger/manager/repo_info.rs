//! plan_ref:
//!   - 04_repository#repo-catalog-contract

use crate::codec;
use crate::ledger::RepoManager;
use crate::ledger::database::cached_shadow_database;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::{
    PROJECTION_FAULTS, REDB_SCHEMA_VERSION, REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS,
    REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable};
use std::path::Path;

impl RepoManager {
    pub fn get_repo_info(&self) -> Result<Option<RepoInfo>> {
        self.run_on_primary_local_repo(Self::read_local_repo_info_from_db)
    }

    pub(crate) fn read_repo_info_from_db(db: &Database) -> Result<Option<RepoInfo>> {
        Self::validate_repo_schema_version(db)?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(REPO_METADATA)?;
        if let Some(guard) = table.get(&REPO_INFO_METADATA_KEY)? {
            let info: RepoInfo = codec::decode(guard.value())?;
            return Ok(Some(info));
        }
        Ok(None)
    }

    pub(crate) fn validate_repo_schema_version(db: &Database) -> Result<()> {
        let read_txn = db.begin_read()?;
        let table = match read_txn.open_table(REPO_METADATA) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                anyhow::bail!(
                    "Unsupported redb schema while reading repo metadata: schema version missing; reset, repair, or migrate this pre-stable repo explicitly"
                );
            }
            Err(err) => return Err(err.into()),
        };
        let Some(version_guard) = table.get(&REPO_SCHEMA_VERSION_METADATA_KEY)? else {
            anyhow::bail!(
                "Unsupported redb schema while reading repo metadata: schema version missing; reset, repair, or migrate this pre-stable repo explicitly"
            );
        };
        let schema_version: u16 = codec::decode(version_guard.value())?;
        if schema_version != REDB_SCHEMA_VERSION {
            anyhow::bail!(
                "Unsupported redb schema version {} while reading repo metadata; expected {}",
                schema_version,
                REDB_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    /// Validates the complete local-authority schema profile without mutating the database.
    ///
    /// Shadow databases deliberately use only `validate_repo_schema_version`: Remote Import
    /// workflow state is host-local authority and must never be created in a shadow database.
    pub(crate) fn validate_local_repo_schema(db: &Database) -> Result<()> {
        Self::validate_repo_schema_version(db)?;
        let read_txn = db.begin_read()?;
        require_local_table(
            read_txn.open_table(REMOTE_IMPORT_SESSIONS),
            "remote_import_sessions",
        )?;
        require_local_table(
            read_txn.open_table(REMOTE_IMPORT_RUNTIME),
            "remote_import_runtime",
        )?;
        require_local_table(read_txn.open_table(PROJECTION_FAULTS), "projection_faults")?;
        Ok(())
    }

    pub(crate) fn read_local_repo_info_from_db(db: &Database) -> Result<Option<RepoInfo>> {
        Self::validate_local_repo_schema(db)?;
        Self::read_repo_info_from_db(db)
    }

    pub(crate) fn read_shadow_repo_info_from_path(path: &Path) -> Result<Option<RepoInfo>> {
        let db = cached_shadow_database(path)?;
        Self::read_repo_info_from_db(db.as_ref())
    }

    pub(crate) fn repo_stem_from_path(path: &Path, context: &str) -> Result<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .filter(|stem| !stem.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(
                    "Broken repo entry {:?} while {}: invalid file stem",
                    path,
                    context
                )
            })
    }

    pub(crate) fn initialize_repo_info_in_new_db(db: &Database, info: &RepoInfo) -> Result<()> {
        validate_local_workflow_tables(db)?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(REPO_METADATA)?;
            if table.get(&REPO_SCHEMA_VERSION_METADATA_KEY)?.is_some()
                || table.get(&REPO_INFO_METADATA_KEY)?.is_some()
            {
                anyhow::bail!("Refusing to initialize repository metadata in a non-empty database");
            }
            let version = codec::encode(&REDB_SCHEMA_VERSION)?;
            table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
            let bytes = codec::encode(info)?;
            table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub(crate) fn write_local_repo_info_to_db(db: &Database, info: &RepoInfo) -> Result<()> {
        Self::validate_local_repo_schema(db)?;
        Self::write_repo_info_to_db(db, info)
    }

    pub(crate) fn write_repo_info_to_db(db: &Database, info: &RepoInfo) -> Result<()> {
        Self::validate_repo_schema_version(db)?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(REPO_METADATA)?;
            let bytes = codec::encode(info)?;
            table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

fn validate_local_workflow_tables(db: &Database) -> Result<()> {
    let read_txn = db.begin_read()?;
    require_local_table(
        read_txn.open_table(REMOTE_IMPORT_SESSIONS),
        "remote_import_sessions",
    )?;
    require_local_table(
        read_txn.open_table(REMOTE_IMPORT_RUNTIME),
        "remote_import_runtime",
    )?;
    require_local_table(read_txn.open_table(PROJECTION_FAULTS), "projection_faults")?;
    Ok(())
}

fn require_local_table<T>(
    result: std::result::Result<T, redb::TableError>,
    name: &str,
) -> Result<()> {
    match result {
        Ok(_) => Ok(()),
        Err(redb::TableError::TableDoesNotExist(_)) => anyhow::bail!(
            "Incomplete redb local-authority schema v{}: required workflow table {} is missing; reset, repair, or migrate this pre-stable repo explicitly",
            REDB_SCHEMA_VERSION,
            name
        ),
        Err(err) => Err(err.into()),
    }
}
