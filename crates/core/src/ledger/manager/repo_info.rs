use crate::ledger::RepoManager;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::REPO_METADATA;
use anyhow::Result;
use redb::Database;

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
}
