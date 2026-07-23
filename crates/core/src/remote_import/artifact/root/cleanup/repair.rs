//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair
//!   - 06_backup#remote-import-removal-owner-plan
//!
//! Exact retry admission for Remote Import owner cleanup.

use super::{
    RemoteImportArtifactRemovalCheckpoint, RemoteImportArtifactRemovalCheckpointState,
    RemoteImportArtifactRemovalPlan, RemoteImportArtifactRoot, RemoteImportResult,
};

impl RemoteImportArtifactRoot {
    pub(in crate::remote_import) fn repo_removal_repair_retry_is_exact(
        plan: &RemoteImportArtifactRemovalPlan,
        checkpoint: &RemoteImportArtifactRemovalCheckpoint,
    ) -> RemoteImportResult<bool> {
        let exact = match &checkpoint.state {
            RemoteImportArtifactRemovalCheckpointState::Prepared => {
                match plan.root_quarantine.observe_cut() {
                    Ok(Some(root)) => {
                        root.original_path_is_absent()? && root.is_quarantined_exact()?
                    }
                    Ok(None) => Self::revalidate_repo_removal(plan)?,
                    Err(_) => false,
                }
            }
            RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root } => {
                root.belongs_to(&plan.root_quarantine)
                    && root.original_path_is_absent()?
                    && root.is_quarantined_exact()?
            }
            RemoteImportArtifactRemovalCheckpointState::RootDeleted { root } => {
                root.belongs_to(&plan.root_quarantine) && root.is_deleted()?
            }
        };
        Ok(exact)
    }
}
