//! plan_ref:
//!   - 04_repository#repo-catalog-contract

use crate::codec;
use crate::ledger::RepoManager;
use crate::ledger::database::cached_database;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use anyhow::{Result, anyhow};
use redb::Database;
use std::path::Path;

impl RepoManager {
    pub fn get_repo_info(&self) -> Result<Option<RepoInfo>> {
        Self::read_repo_info_from_db(&self.local_db)
    }

    pub(crate) fn read_repo_info_from_db(db: &Database) -> Result<Option<RepoInfo>> {
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
        if let Some(guard) = table.get(&REPO_INFO_METADATA_KEY)? {
            let info: RepoInfo = codec::decode(guard.value())?;
            return Ok(Some(info));
        }
        Ok(None)
    }

    pub(crate) fn read_repo_info_from_path(path: &Path) -> Result<Option<RepoInfo>> {
        let db = cached_database(path)?;
        Self::read_repo_info_from_db(db.as_ref())
    }

    pub(crate) fn read_required_repo_info_from_path(
        path: &Path,
        stem: &str,
        context: &str,
    ) -> Result<RepoInfo> {
        Self::read_repo_info_from_path(path)?.ok_or_else(|| {
            anyhow!(
                "Broken local repo {} while {}: repository metadata missing",
                stem,
                context
            )
        })
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

    pub(crate) fn write_repo_info_to_db(db: &Database, info: &RepoInfo) -> Result<()> {
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(REPO_METADATA)?;
            let version = codec::encode(&REDB_SCHEMA_VERSION)?;
            table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
            let bytes = codec::encode(info)?;
            table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
