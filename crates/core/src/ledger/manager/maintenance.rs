//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!
use crate::ledger::manager::types::RepoManager;
use anyhow::Result;

impl RepoManager {
    pub(crate) fn refresh_local_repo_catalog(&self) -> Result<()> {
        Self::validate_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )?;
        Self::validate_local_repo_source_control_tables(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )
    }
}
