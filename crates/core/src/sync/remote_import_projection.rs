//! plan_ref:
//!   - 03_storage/projection#remote-import-projection-writeback
//!   - 06_backup#remote-import-session-contract
//!
//! Remote Import post-commit writeback. Only the paths committed by the
//! immutable session are overwritten; unrelated workspace state is untouched.

use super::{SyncManager, rebuild};
use crate::ledger::range;
use crate::models::GlobalSeq;
use crate::{projection_fault, remote_import};
use anyhow::{Context, Result, anyhow};

impl SyncManager {
    pub(crate) fn recover_pending_remote_import_projections(&self) -> Result<()> {
        for repo_id in remote_import::pending_projection_repo_ids(&self.repo)? {
            let repo_name = self
                .repo
                .resolve_local_repo_name_for_execution(Some(repo_id), None)?;
            let service = remote_import::RemoteImportService::open(&self.repo, repo_id)?;
            if let Err(error) = service.recover_pending_projection(self, &repo_name) {
                tracing::error!(
                    repo_name,
                    %repo_id,
                    %error,
                    "Remote Import Pending projection recovery could not inspect durable state"
                );
                self.mark_remote_import_projection_degraded(&repo_name);
            }
        }
        Ok(())
    }

    pub(crate) fn writeback_remote_import_projection(
        &self,
        repo_name: &str,
        expected_head: GlobalSeq,
        paths: &[String],
    ) -> Result<()> {
        let observed = self.repo.run_on_local_repo(repo_name, range::get_max_seq)?;
        if observed != expected_head.storage_key() {
            return Err(anyhow!(
                "Remote Import projection writeback head changed: expected {}, observed {}",
                expected_head,
                observed
            ));
        }
        let root = self.repo.local_repo_workspace_root(repo_name)?;
        std::fs::create_dir_all(&root)?;
        self.repo.ensure_local_repo_workspace_identity(repo_name)?;
        crate::utils::notegit::ensure_gitignore_ignores_notegit(&root)?;
        for path in paths {
            let doc_id = self
                .repo
                .get_tracked_docid_in_local_repo(repo_name, path)?
                .ok_or_else(|| {
                    anyhow!(
                        "Remote Import committed path has no Ledger document identity: {path:?}"
                    )
                })?;
            let rebuilt = rebuild::rebuild_local_doc_in_repo(&self.repo, repo_name, doc_id)?;
            let target = self.repo.local_repo_workspace_path(repo_name, path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let relative = self.repo.local_repo_workspace_relative(repo_name, path);
            self.persist_guard.record(&relative, &rebuilt.content);
            if let Err(error) = std::fs::write(&target, rebuilt.content) {
                self.persist_guard.clear(&relative);
                return Err(error).with_context(|| {
                    format!("Remote Import projection writeback failed for {path:?}")
                });
            }
            self.repo
                .bind_workspace_inode_in_local_repo(repo_name, path, doc_id)?;
        }
        let after = self.repo.run_on_local_repo(repo_name, range::get_max_seq)?;
        if after != expected_head.storage_key() {
            return Err(anyhow!(
                "Remote Import projection writeback source changed: expected {}, observed {}",
                expected_head,
                after
            ));
        }
        Ok(())
    }

    pub(crate) fn mark_remote_import_projection_degraded(&self, repo_name: &str) {
        match self.repo.get_repo_info_for(None, Some(repo_name)) {
            Ok(Some(info)) => self.mark_projection_degraded_id(info.uuid),
            Ok(None) => tracing::error!(
                repo_name,
                "Remote Import could not mark missing repo projection degraded"
            ),
            Err(error) => tracing::error!(
                repo_name,
                %error,
                "Remote Import could not resolve repo while marking projection degraded"
            ),
        }
    }

    pub(crate) fn reconcile_remote_import_projection_health(&self, repo_name: &str) -> Result<()> {
        let info = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow!("Repository not found: {repo_name}"))?;
        let has_fault = projection_fault::load_degraded_repo_ids(&self.repo)?
            .into_iter()
            .any(|repo_id| repo_id == info.uuid);
        let has_pending = remote_import::pending_projection_repo_ids(&self.repo)?
            .into_iter()
            .any(|repo_id| repo_id == info.uuid);
        if has_fault || has_pending {
            self.mark_projection_degraded_id(info.uuid);
        } else {
            self.clear_projection_degraded(repo_name)?;
        }
        Ok(())
    }
}
