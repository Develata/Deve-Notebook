//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Read-only catalog projections used by product/runtime admission. Dynamic
//! create/remove authority lives in `repo_catalog_runtime`; repository display
//! aliases live in `host_repo_alias` and never rename durable machine paths.

use crate::ledger::RepoCatalogMembershipState;
use crate::ledger::manager::types::{LocalRepoSummary, RepoManager};
use crate::models::RepoId;
use anyhow::{Result, anyhow};

impl RepoManager {
    /// Normal product catalog projection backed only by durable membership
    /// records. Prepared/removed database artifacts never enter this list.
    pub fn list_cataloged_local_repo_summaries(&self) -> Result<Vec<LocalRepoSummary>> {
        let mut summaries = Vec::new();
        for repo_id in self.normal_repo_catalog_ids()? {
            let info = self
                .get_local_repo_info_by_id(repo_id)?
                .ok_or_else(|| anyhow!("Catalog member {repo_id} has no local metadata"))?;
            if info.uuid != repo_id {
                return Err(anyhow!(
                    "Catalog member identity mismatch: expected {repo_id}, got {}",
                    info.uuid
                ));
            }
            let execution_name = repo_id.to_string();
            if info.name != execution_name {
                return Err(anyhow!(
                    "Catalog member {repo_id} has non-canonical machine name {:?}",
                    info.name
                ));
            }
            summaries.push(LocalRepoSummary {
                repo_id,
                name: execution_name.clone(),
                execution_name,
            });
        }
        summaries.sort_by_key(|summary| summary.repo_id);
        Ok(summaries)
    }

    /// Returns whether a RepoId is excluded from the active catalog. Missing
    /// records are non-members, so prepared artifacts cannot regain admission
    /// through a filesystem scan.
    pub fn is_local_repo_removed(&self, repo_id: RepoId) -> Result<bool> {
        Ok(!matches!(
            self.repo_catalog_membership_record(repo_id)?,
            Some(record) if record.state() == RepoCatalogMembershipState::Normal
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_listing_is_catalog_only_and_uuid_named() -> anyhow::Result<()> {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let projection = dir.path().join("projection");
        let repo_id = RepoId::new_v4();
        let repo = RepoManager::init_with_options(
            &ledger,
            4,
            Some(&repo_id.to_string()),
            crate::ledger::init::RepoInitOptions {
                repo_id: Some(repo_id),
                repo_url: None,
            },
        )?;
        let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, &projection)?;
        let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
        std::fs::create_dir_all(&workspace)?;
        crate::utils::notegit::ensure_repo_identity_marker(
            &workspace,
            repo_id,
            &repo_id.to_string(),
        )?;
        repo.seed_catalog_membership_from_records()?;
        let authority = repo.claim_repo_catalog_cut_authority()?;
        let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
        let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
        let permit = authority.permit(repo_id)?;
        repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
        let summaries = repo.list_cataloged_local_repo_summaries()?;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, summaries[0].repo_id.to_string());
        assert_eq!(
            summaries[0].execution_name,
            summaries[0].repo_id.to_string()
        );
        Ok(())
    }
}
