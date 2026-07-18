//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/projection#durable-projection-fault-contract

use super::{DirRefreshGuard, ProjectionHealth, SyncManager, materialize, scan};
use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::projection_fault;
use crate::remote_import;
use crate::vfs::Vfs;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

impl SyncManager {
    pub fn new_checked(repo: Arc<RepoManager>) -> Result<Self> {
        repo.list_local_repo_names_for_execution()?;
        repo.validate_projection_locator_map()?;
        let manager = Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vfs: Vfs::unrooted(),
            projection_health: ProjectionHealth::new(),
        };
        manager.load_durable_projection_faults()?;
        Ok(manager)
    }

    pub fn scan(&self) -> Result<()> {
        let mut degraded_ids = projection_recovery_repo_ids(&self.repo)?;
        let mut degraded = self.execution_names_for_repo_ids(&degraded_ids)?;
        let durable_degraded: HashSet<String> = degraded.iter().cloned().collect();
        let materialize_degraded = materialize::prepare_local_workspaces(
            &self.repo,
            &self.persist_guard,
            &durable_degraded,
        )?;
        for repo_name in materialize_degraded {
            if let Some(info) = self.repo.get_repo_info_for(None, Some(&repo_name))? {
                degraded_ids.push(info.uuid);
                degraded.push(repo_name);
            }
        }
        degraded_ids.sort();
        degraded_ids.dedup();
        degraded.sort();
        degraded.dedup();
        let degraded_set = degraded.iter().cloned().collect();
        self.replace_projection_degraded(&degraded_ids);
        scan::scan_all_local_repos_excluding(&self.repo, &self.vfs, &degraded_set)
    }

    pub fn scan_repo(&self, repo_name: &str) -> Result<()> {
        let info = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow::anyhow!("Repository not found: {repo_name}"))?;
        let degraded = projection_recovery_repo_ids(&self.repo)?;
        if degraded.contains(&info.uuid) {
            anyhow::bail!("Projection workspace for repo {repo_name} is degraded; scan aborted");
        }
        scan::scan_local_repo(&self.repo, &self.vfs, repo_name)
    }

    fn load_durable_projection_faults(&self) -> Result<()> {
        let degraded = projection_recovery_repo_ids(&self.repo)?;
        for repo_id in degraded {
            self.mark_projection_degraded_id(repo_id);
        }
        Ok(())
    }

    fn execution_names_for_repo_ids(&self, repo_ids: &[RepoId]) -> Result<Vec<String>> {
        repo_ids
            .iter()
            .map(|repo_id| {
                self.repo
                    .resolve_local_repo_name_for_execution(Some(*repo_id), None)
            })
            .collect()
    }
}

fn projection_recovery_repo_ids(repo: &RepoManager) -> Result<Vec<RepoId>> {
    let mut degraded = projection_fault::load_degraded_repo_ids(repo)?;
    degraded.extend(remote_import::pending_projection_repo_ids(repo)?);
    degraded.sort();
    degraded.dedup();
    Ok(degraded)
}

#[cfg(test)]
mod tests {
    use super::SyncManager;
    use crate::ledger::{RepoManager, init::RepoInitOptions};
    use crate::remote_import::{
        RemoteImportApplyRequest, RemoteImportBaseline, RemoteImportDigest,
        RemoteImportPrepareRequest, RemoteImportProjectionOutcome, RemoteImportRuntime,
    };
    use std::collections::BTreeMap;
    use std::io::Cursor;
    use std::sync::Arc;

    #[test]
    fn startup_marks_applied_pending_receipt_degraded_before_product_recovery() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let repo_id = uuid::Uuid::new_v4();
        let mut repo = RepoManager::init_with_options(
            &ledger,
            8,
            Some("notes"),
            RepoInitOptions {
                repo_id: Some(repo_id),
                repo_url: None,
            },
        )?;
        repo.set_projection_base_for_all_local_repos_checked(dir.path().join("projection"))?;
        let runtime = RemoteImportRuntime::open(&repo, repo_id)?;
        let locator = RemoteImportDigest::of(b"locator");
        let mut capture = runtime.begin_prepare(RemoteImportPrepareRequest {
            source_binding_digest: RemoteImportDigest::of(b"source"),
            locator_binding_digest: locator,
            baseline: RemoteImportBaseline {
                ledger_head: 0.into(),
                ignore_digest: RemoteImportDigest::of(b"ignore"),
                locator_digest: locator,
                existing: BTreeMap::new(),
            },
        })?;
        capture.capture_file("pending.md", Cursor::new(b"pending"))?;
        let ready = capture.finish()?;
        let candidate = ready.candidate.as_ref().expect("candidate");
        let prepared = runtime.prepare_apply(
            &repo,
            repo.local_repo_name(),
            RemoteImportApplyRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: ready.session_id,
                revision: candidate.revision,
                locator_digest: candidate.locator_digest,
                ignore_digest: candidate.ignore_digest,
            },
        )?;
        let receipt = runtime.commit_apply(&repo, repo.local_repo_name(), prepared)?;
        assert_eq!(
            receipt.projection_outcome,
            RemoteImportProjectionOutcome::Pending
        );

        let repo_name = repo.local_repo_name().to_string();
        let sync = SyncManager::new_checked(Arc::new(repo))?;
        assert!(sync.is_projection_degraded(&repo_name));
        Ok(())
    }
}
