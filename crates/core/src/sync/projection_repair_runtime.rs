//! plan_ref:
//!   - 04_repository#repo-health-and-repair
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract

use super::{
    ProjectionDiagnostic, SyncManager, projection_diagnostic, projection_fault_journal,
    rebuild_projection,
};
use crate::ledger::RepoManager;
use crate::models::DocId;
use anyhow::Result;

impl SyncManager {
    /// 显式强制重建指定 repo 的 Projection Workspace。
    pub fn rebuild_projection_local_repo(&self, repo_name: &str) -> Result<()> {
        if let Err(err) =
            rebuild_projection::rebuild_local_repo(&self.repo, &self.persist_guard, repo_name)
        {
            self.record_projection_fault(
                repo_name,
                projection_fault_journal::ProjectionFaultInput {
                    fault_kind:
                        projection_fault_journal::ProjectionFaultKind::ProjectionRebuildInterrupted,
                    target_path: None,
                    source_path: None,
                    doc_id: None,
                    ledger_seq_or_head: None,
                    last_error: &err.to_string(),
                },
            );
            return Err(err);
        }
        projection_fault_journal::clear_faults_for_repo(&self.repo, repo_name)?;
        self.clear_projection_degraded(repo_name);
        Ok(())
    }

    pub fn diagnose_projection_local_repo(&self, repo_name: &str) -> Result<ProjectionDiagnostic> {
        projection_diagnostic::diagnose(&self.repo, repo_name)
    }

    pub fn is_projection_degraded(&self, repo_name: &str) -> bool {
        self.projection_health.is_degraded(repo_name)
    }

    pub fn mark_projection_writeback_fault(&self, repo_name: &str) {
        self.record_projection_fault(
            repo_name,
            projection_fault_journal::ProjectionFaultInput {
                fault_kind:
                    projection_fault_journal::ProjectionFaultKind::ProjectionWritebackFailed,
                target_path: None,
                source_path: None,
                doc_id: None,
                ledger_seq_or_head: None,
                last_error: "projection writeback failed",
            },
        );
    }

    pub(super) fn mark_projection_writeback_fault_for_doc(
        &self,
        repo_name: &str,
        doc_id: DocId,
        target_path: Option<&str>,
        err: &anyhow::Error,
    ) {
        self.record_projection_fault(
            repo_name,
            projection_fault_journal::ProjectionFaultInput {
                fault_kind:
                    projection_fault_journal::ProjectionFaultKind::ProjectionWritebackFailed,
                target_path,
                source_path: None,
                doc_id: Some(doc_id),
                ledger_seq_or_head: None,
                last_error: &err.to_string(),
            },
        );
    }

    pub(super) fn mark_projection_writeback_fault_for_path(
        &self,
        repo_name: &str,
        target_path: &str,
        err: &anyhow::Error,
    ) {
        self.record_projection_fault(
            repo_name,
            projection_fault_journal::ProjectionFaultInput {
                fault_kind:
                    projection_fault_journal::ProjectionFaultKind::ProjectionWritebackFailed,
                target_path: (!target_path.is_empty()).then_some(target_path),
                source_path: None,
                doc_id: None,
                ledger_seq_or_head: None,
                last_error: &err.to_string(),
            },
        );
    }

    pub fn healthy_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        let degraded = self
            .projection_health
            .degraded_snapshot()
            .map_err(anyhow::Error::msg)?;
        Ok(self
            .repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter(|repo_name| !degraded.contains(repo_name))
            .collect())
    }

    pub fn degraded_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        let degraded = self
            .projection_health
            .degraded_snapshot()
            .map_err(anyhow::Error::msg)?;
        Ok(self
            .repo
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter(|repo_name| degraded.contains(repo_name))
            .collect())
    }

    pub(super) fn replace_projection_degraded(&self, repo_names: &[String]) {
        self.projection_health.replace_degraded(repo_names);
    }

    pub(super) fn mark_projection_degraded(&self, repo_name: &str) {
        self.projection_health.mark_degraded(repo_name);
    }

    pub(super) fn clear_projection_degraded(&self, repo_name: &str) {
        self.projection_health.clear_degraded(repo_name);
    }

    fn record_projection_fault(
        &self,
        repo_name: &str,
        input: projection_fault_journal::ProjectionFaultInput<'_>,
    ) {
        self.mark_projection_degraded(repo_name);
        if let Err(err) = projection_fault_journal::record_fault(&self.repo, repo_name, input) {
            tracing::error!(
                repo_name = %repo_name,
                error = %err,
                "Failed to persist durable projection fault"
            );
        }
    }
}

pub fn diagnose_projection_local_repo(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<ProjectionDiagnostic> {
    projection_diagnostic::diagnose(repo, repo_name)
}

pub(crate) fn diagnose_projection_local_repo_stem(
    repo: &RepoManager,
    repo_name: &str,
    repo_stem: &str,
) -> Result<ProjectionDiagnostic> {
    projection_diagnostic::diagnose_stem(repo, repo_name, repo_stem)
}
