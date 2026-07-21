//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-state-machine
//!
//! Authority-backed database leasing for the Remote Import store. Production
//! callers retain only a membership-bound capability; every transaction takes
//! a short, revocable per-RepoId lease.

use crate::ledger::manager::{BoundRepoAuthority, RepoAuthorityLease};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use redb::Database;
#[cfg(test)]
use std::sync::Arc;

pub(super) enum StoreDatabase {
    Authority(BoundRepoAuthority),
    #[cfg(test)]
    TestAuthority(RepoAuthorityLease),
    #[cfg(test)]
    Test(Arc<Database>),
}

#[cfg(test)]
#[derive(Clone)]
pub(in crate::remote_import) struct RemoteImportTestDatabase(pub(super) Arc<StoreDatabase>);

#[cfg(test)]
impl RemoteImportTestDatabase {
    pub(in crate::remote_import) fn from_authority(lease: RepoAuthorityLease) -> Self {
        Self(Arc::new(StoreDatabase::TestAuthority(lease)))
    }
}

#[cfg(test)]
impl From<Arc<Database>> for RemoteImportTestDatabase {
    fn from(db: Arc<Database>) -> Self {
        Self(Arc::new(StoreDatabase::Test(db)))
    }
}

#[cfg(test)]
impl std::ops::Deref for RemoteImportTestDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().deref()
    }
}

#[cfg(test)]
impl AsRef<Database> for RemoteImportTestDatabase {
    fn as_ref(&self) -> &Database {
        self
    }
}

impl std::ops::Deref for StoreDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Authority(_) => {
                unreachable!("production bound authority is not a test database handle")
            }
            #[cfg(test)]
            Self::TestAuthority(lease) => lease.db(),
            #[cfg(test)]
            Self::Test(db) => db.as_ref(),
        }
    }
}

pub(in crate::remote_import) enum StoreDatabaseLease<'a> {
    Authority(RepoAuthorityLease, std::marker::PhantomData<&'a ()>),
    #[cfg(test)]
    TestAuthority(&'a RepoAuthorityLease),
    #[cfg(test)]
    Test(&'a Database),
}

impl std::ops::Deref for StoreDatabaseLease<'_> {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Authority(lease, _) => lease.db(),
            #[cfg(test)]
            Self::TestAuthority(lease) => lease.db(),
            #[cfg(test)]
            Self::Test(db) => db,
        }
    }
}

impl StoreDatabase {
    pub(super) fn lease(&self) -> RemoteImportResult<StoreDatabaseLease<'_>> {
        match self {
            Self::Authority(authority) => authority
                .lease()
                .map(|lease| StoreDatabaseLease::Authority(lease, std::marker::PhantomData))
                .map_err(RemoteImportError::storage),
            #[cfg(test)]
            Self::TestAuthority(lease) => Ok(StoreDatabaseLease::TestAuthority(lease)),
            #[cfg(test)]
            Self::Test(db) => Ok(StoreDatabaseLease::Test(db.as_ref())),
        }
    }
}
