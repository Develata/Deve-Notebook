//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! Persistent per-RepoId owner lock with explicit final release.

use super::{LocalAuthorityError, RepoId};
use std::fs::{File, TryLockError};

pub(super) struct RepoAuthorityLock {
    file: File,
    repo_id: RepoId,
    locked: bool,
}

impl RepoAuthorityLock {
    pub(super) fn acquire(file: File, repo_id: RepoId) -> Result<Self, LocalAuthorityError> {
        match file.try_lock() {
            Ok(()) => Ok(Self {
                file,
                repo_id,
                locked: true,
            }),
            Err(TryLockError::WouldBlock) => Err(LocalAuthorityError::Busy(repo_id)),
            Err(TryLockError::Error(error)) => Err(LocalAuthorityError::Io(error)),
        }
    }

    pub(super) fn file(&self) -> &File {
        &self.file
    }

    pub(super) fn release(mut self) -> Result<(), (Self, std::io::Error)> {
        match self.file.unlock() {
            Ok(()) => {
                self.locked = false;
                Ok(())
            }
            Err(error) => Err((self, error)),
        }
    }
}

impl Drop for RepoAuthorityLock {
    fn drop(&mut self) {
        if self.locked
            && let Err(error) = self.file.unlock()
        {
            tracing::error!(
                repo_id = %self.repo_id,
                %error,
                "failed to unlock local authority owner file"
            );
        }
    }
}
