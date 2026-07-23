//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Exact conditional cleanup of a removed repo's host-local locator binding.

use super::store::ProjectionLocatorMapGuard;
use super::{ProjectionLocatorRecord, RepoManager};
use crate::models::RepoId;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionLocatorRemovalPlan {
    record: ProjectionLocatorRecord,
}

impl ProjectionLocatorRemovalPlan {
    pub const fn repo_id(&self) -> RepoId {
        self.record.repo_id
    }

    pub fn record(&self) -> &ProjectionLocatorRecord {
        &self.record
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionLocatorCleanupDisposition {
    Deleted,
    AlreadyAbsent,
}

impl RepoManager {
    pub fn prepare_projection_locator_removal(
        &self,
        repo_id: RepoId,
    ) -> Result<ProjectionLocatorRemovalPlan> {
        Ok(ProjectionLocatorRemovalPlan {
            record: self.validated_projection_locator_for_repo_id(repo_id)?,
        })
    }

    pub fn cleanup_projection_locator_removal(
        &self,
        plan: &ProjectionLocatorRemovalPlan,
    ) -> Result<ProjectionLocatorCleanupDisposition> {
        let _map_guard = ProjectionLocatorMapGuard::acquire(&self.ledger_dir)?;
        let catalog = self
            .repo_catalog_membership_record(plan.repo_id())?
            .ok_or_else(|| anyhow!("removed repo catalog tombstone is missing"))?;
        if catalog.state() != crate::ledger::RepoCatalogMembershipState::Removed {
            return Err(anyhow!(
                "projection locator cleanup requires Removed membership"
            ));
        }
        let mut file = self.read_projection_locator_file()?;
        let Some(current) = file
            .locators
            .iter()
            .find(|record| record.repo_id == plan.repo_id())
            .cloned()
        else {
            return Ok(ProjectionLocatorCleanupDisposition::AlreadyAbsent);
        };
        if current != plan.record {
            return Err(anyhow!("projection locator changed before exact cleanup"));
        }
        file.locators
            .retain(|record| record.repo_id != plan.repo_id());
        self.validate_projection_locator_records(&file.locators, false)?;
        self.write_projection_locator_file(&file)?;
        Ok(ProjectionLocatorCleanupDisposition::Deleted)
    }

    pub fn projection_locator_removal_is_absent(
        &self,
        plan: &ProjectionLocatorRemovalPlan,
    ) -> Result<bool> {
        Ok(self
            .query_projection_locator_record_for_repo_id(plan.repo_id())?
            .is_none())
    }

    pub fn projection_locator_removal_retry_is_exact(
        &self,
        plan: &ProjectionLocatorRemovalPlan,
    ) -> Result<bool> {
        Ok(
            match self.query_projection_locator_record_for_repo_id(plan.repo_id())? {
                Some(current) => current == plan.record,
                None => true,
            },
        )
    }
}
