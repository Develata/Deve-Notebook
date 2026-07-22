//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Fail-closed abandonment behavior for committed cleanup capabilities.

use super::super::{RepoAuthorityCleanupGuard, RepoAuthoritySlot};

impl Drop for RepoAuthorityCleanupGuard {
    fn drop(&mut self) {
        if !self.settled {
            // A committed cleanup may only release its owner lock after the
            // owner-specific receipt and exact tombstone retirement succeed.
            // Leaving the slot untouched is intentionally fail-closed.
            tracing::error!(repo_id = %self.repo_id, generation = self.generation, "committed local authority cleanup guard dropped before completion; owner lock retained");
            if let Ok(mut slots) = self.inner.slots.lock()
                && let Some(RepoAuthoritySlot::CommittedCleanup {
                    generation,
                    db_path,
                    cleanup_capability_issued,
                    ..
                }) = slots.get_mut(&self.repo_id)
                && *generation == self.generation
                && *db_path == self.db_path
            {
                *cleanup_capability_issued = false;
            }
        }
    }
}
