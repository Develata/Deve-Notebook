use crate::ledger::RepoManager;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::REPO_METADATA;
use anyhow::Result;
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
        let db = Database::create(path)?;
        Self::read_repo_info_from_db(&db)
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
