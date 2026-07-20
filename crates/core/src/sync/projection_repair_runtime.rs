//! plan_ref:
//!   - 04_repository#repo-health-and-repair
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/projection#durable-projection-fault-contract

use super::{ProjectionDiagnostic, SyncManager, projection_diagnostic, rebuild_projection};
use crate::ledger::RepoManager;
use crate::models::{DocId, RepoId};
use crate::projection_fault::{self, ProjectionFaultInput, ProjectionFaultKind};
use anyhow::Result;

impl SyncManager {
    /// 显式强制重建指定 repo 的 Projection Workspace。
    pub fn rebuild_projection_local_repo(&self, repo_name: &str) -> Result<()> {
        if let Err(err) =
            rebuild_projection::rebuild_local_repo(&self.repo, &self.persist_guard, repo_name)
        {
            if let Err(fault_error) = self.record_projection_fault(
                repo_name,
                ProjectionFaultInput {
                    fault_kind: ProjectionFaultKind::ProjectionRebuildInterrupted,
                    target_path: None,
                    source_path: None,
                    doc_id: None,
                    ledger_seq_or_head: None,
                    last_error: &err.to_string(),
                },
            ) {
                return Err(err.context(format!(
                    "failed to persist Projection Fault evidence: {fault_error}"
                )));
            }
            return Err(err);
        }
        projection_fault::clear_faults_for_repo(&self.repo, repo_name)?;
        self.clear_projection_degraded(repo_name)?;
        Ok(())
    }

    pub fn diagnose_projection_local_repo(&self, repo_name: &str) -> Result<ProjectionDiagnostic> {
        projection_diagnostic::diagnose(&self.repo, repo_name)
    }

    pub fn is_projection_degraded(&self, repo_name: &str) -> bool {
        self.repo
            .get_repo_info_for(None, Some(repo_name))
            .ok()
            .flatten()
            .map(|info| self.projection_health.is_degraded(info.uuid))
            .unwrap_or(true)
    }

    pub fn mark_projection_writeback_fault(&self, repo_name: &str) -> Result<()> {
        self.record_projection_fault(
            repo_name,
            ProjectionFaultInput {
                fault_kind: ProjectionFaultKind::ProjectionWritebackFailed,
                target_path: None,
                source_path: None,
                doc_id: None,
                ledger_seq_or_head: None,
                last_error: "projection writeback failed",
            },
        )
    }

    pub(super) fn mark_projection_writeback_fault_for_doc(
        &self,
        repo_name: &str,
        doc_id: DocId,
        target_path: Option<&str>,
        err: &anyhow::Error,
    ) -> Result<()> {
        self.record_projection_fault(
            repo_name,
            ProjectionFaultInput {
                fault_kind: ProjectionFaultKind::ProjectionWritebackFailed,
                target_path,
                source_path: None,
                doc_id: Some(doc_id),
                ledger_seq_or_head: None,
                last_error: &err.to_string(),
            },
        )
    }

    pub(super) fn mark_projection_writeback_fault_for_path(
        &self,
        repo_name: &str,
        target_path: &str,
        err: &anyhow::Error,
    ) -> Result<()> {
        self.record_projection_fault(
            repo_name,
            ProjectionFaultInput {
                fault_kind: ProjectionFaultKind::ProjectionWritebackFailed,
                target_path: (!target_path.is_empty()).then_some(target_path),
                source_path: None,
                doc_id: None,
                ledger_seq_or_head: None,
                last_error: &err.to_string(),
            },
        )
    }

    pub(super) fn mark_projection_rebuild_fault(
        &self,
        repo_name: &str,
        err: &anyhow::Error,
    ) -> Result<()> {
        self.record_projection_fault(
            repo_name,
            ProjectionFaultInput {
                fault_kind: ProjectionFaultKind::ProjectionRebuildInterrupted,
                target_path: None,
                source_path: None,
                doc_id: None,
                ledger_seq_or_head: None,
                last_error: &err.to_string(),
            },
        )
    }

    pub fn healthy_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        let degraded = self
            .projection_health
            .degraded_snapshot()
            .map_err(anyhow::Error::msg)?;
        self.repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter_map(
                |repo_name| match self.repo.get_repo_info_for(None, Some(&repo_name)) {
                    Ok(Some(info)) if !degraded.contains(&info.uuid) => Some(Ok(repo_name)),
                    Ok(Some(_)) => None,
                    Ok(None) => Some(Err(anyhow::anyhow!("Repository not found: {repo_name}"))),
                    Err(error) => Some(Err(error)),
                },
            )
            .collect()
    }

    pub fn degraded_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        let degraded = self
            .projection_health
            .degraded_snapshot()
            .map_err(anyhow::Error::msg)?;
        self.repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter_map(
                |repo_name| match self.repo.get_repo_info_for(None, Some(&repo_name)) {
                    Ok(Some(info)) if degraded.contains(&info.uuid) => Some(Ok(repo_name)),
                    Ok(Some(_)) => None,
                    Ok(None) => Some(Err(anyhow::anyhow!("Repository not found: {repo_name}"))),
                    Err(error) => Some(Err(error)),
                },
            )
            .collect()
    }

    pub(super) fn replace_projection_degraded(&self, repo_ids: &[RepoId]) {
        self.projection_health.replace_degraded(repo_ids);
    }

    pub(super) fn mark_projection_degraded_id(&self, repo_id: RepoId) {
        self.projection_health.mark_degraded(repo_id);
    }

    pub(super) fn clear_projection_degraded(&self, repo_name: &str) -> Result<()> {
        let info = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow::anyhow!("Repository not found: {repo_name}"))?;
        self.projection_health.clear_degraded(info.uuid);
        Ok(())
    }

    fn record_projection_fault(
        &self,
        repo_name: &str,
        input: ProjectionFaultInput<'_>,
    ) -> Result<()> {
        let info = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow::anyhow!("Repository not found: {repo_name}"))?;
        if let Err(error) = projection_fault::record_fault(&self.repo, repo_name, input) {
            self.mark_projection_degraded_id(info.uuid);
            return Err(error.into());
        }
        self.mark_projection_degraded_id(info.uuid);
        Ok(())
    }
}

pub fn diagnose_projection_local_repo(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<ProjectionDiagnostic> {
    projection_diagnostic::diagnose(repo, repo_name)
}
