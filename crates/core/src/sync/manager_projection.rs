//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract

use super::{
    ProjectionDiagnostic, SyncManager, materialize, projection_diagnostic, rebuild_projection,
};
use crate::ledger::RepoManager;
use anyhow::Result;

impl SyncManager {
    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        match materialize::materialize_local_repo(&self.repo, &self.persist_guard, repo_name) {
            Ok(()) => {
                self.clear_projection_degraded(repo_name);
                Ok(())
            }
            Err(err) => {
                if materialize::is_broken_structure_projection_error(&err) {
                    self.mark_projection_degraded(repo_name);
                }
                Err(err)
            }
        }
    }

    /// 显式强制重建指定 repo 的 Vault projection。
    pub fn rebuild_projection_local_repo(&self, repo_name: &str) -> Result<()> {
        rebuild_projection::rebuild_local_repo(&self.repo, &self.persist_guard, repo_name)?;
        self.clear_projection_degraded(repo_name);
        Ok(())
    }

    pub fn diagnose_projection_local_repo(&self, repo_name: &str) -> Result<ProjectionDiagnostic> {
        projection_diagnostic::diagnose(&self.repo, repo_name)
    }

    pub fn is_projection_degraded(&self, repo_name: &str) -> bool {
        self.projection_health.is_degraded(repo_name)
    }

    pub fn healthy_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        Ok(self
            .repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter(|repo_name| !self.is_projection_degraded(repo_name))
            .collect())
    }

    pub fn degraded_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        Ok(self
            .repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter(|repo_name| self.is_projection_degraded(repo_name))
            .collect())
    }

    pub(super) fn replace_projection_degraded(&self, repo_names: &[String]) {
        self.projection_health.replace_degraded(repo_names);
    }

    fn mark_projection_degraded(&self, repo_name: &str) {
        self.projection_health.mark_degraded(repo_name);
    }

    fn clear_projection_degraded(&self, repo_name: &str) {
        self.projection_health.clear_degraded(repo_name);
    }
}

pub fn diagnose_projection_local_repo(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<ProjectionDiagnostic> {
    projection_diagnostic::diagnose(repo, repo_name)
}
