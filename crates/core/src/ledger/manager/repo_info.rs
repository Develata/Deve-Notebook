use crate::ledger::RepoManager;
use crate::ledger::database::cached_database;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::REPO_METADATA;
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
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        if let Some(guard) = table.get(&0)? {
            let info: RepoInfo = bincode::deserialize(guard.value())?;
            return Ok(Some(info));
        }
        Ok(None)
    }

    pub(crate) fn read_repo_info_from_path(path: &Path) -> Result<Option<RepoInfo>> {
        let db = cached_database(path)?;
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

    pub(crate) fn write_repo_info_to_db(db: &Database, info: &RepoInfo) -> Result<()> {
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(REPO_METADATA)?;
            let bytes = bincode::serialize(info)?;
            table.insert(&0, bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
