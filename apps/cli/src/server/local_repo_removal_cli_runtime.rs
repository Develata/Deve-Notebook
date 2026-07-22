//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#local-repo-removal-contract
//!   - 14_commands#repo-removal-command-contract
//!
//! Narrow server-runtime facade for offline CLI removal. It deliberately
//! exposes only typed Prepare/Execute/Status operations, while composing the
//! same owner runtimes used by the long-lived server.

use super::repo_mutation::RepoMutationPublicationGate;
use super::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleHostExecutor, RepoLifecycleHostPublicationSink, RepoLifecycleJobError,
    RepoLifecycleJobOutcome, RepoLifecycleJobPhase, RepoLifecycleJobRuntime,
    RepoRemovalExecuteIntent, RepoRemovalIssuerBinding, RepoRemovalPrepareIntent,
};
use super::runtime::repo_lifecycle_runtime::RepoLifecycleCoordinator;
use super::runtime::repo_session_runtime::RepoSessionRuntime;
use super::runtime::watcher_runtime::{WatcherSupervisor, start_file_watchers};
use crate::remote_import_runtime::RemoteImportCoordinator;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use deve_core::protocol::{
    LocalRepoRemovalPreview, RemovalConfirmationToken, RepoLifecycleOutcome, RepoLifecycleState,
    ServerMessage,
};
use deve_core::remote_import::RemoteImportService;
use deve_core::sync::SyncManager;
use deve_core::utils::fs::{HostPathIdentity, HostPathKind};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

const SCOPE_NONCE: u64 = 1;
const SWITCH_NONCE: u64 = 2;

pub(crate) struct OfflineRemovalPrepared {
    pub(crate) preparation_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) preview: LocalRepoRemovalPreview,
    pub(crate) confirmation_token: Option<RemovalConfirmationToken>,
}

pub(crate) struct OfflineRemovalAccepted {
    pub(crate) request_id: Uuid,
    pub(crate) job_id: Uuid,
    pub(crate) repo_id: RepoId,
}

pub(crate) struct OfflineRemovalStatus {
    pub(crate) request_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) state: RepoLifecycleState,
    pub(crate) outcome: Option<RepoLifecycleOutcome>,
    pub(crate) publication_pending: bool,
}

pub(crate) struct OfflineRemovalRuntime {
    repo: Arc<RepoManager>,
    jobs: Arc<RepoLifecycleJobRuntime>,
    watchers: Arc<WatcherSupervisor>,
}

impl OfflineRemovalRuntime {
    pub(crate) fn start(repo: Arc<RepoManager>) -> Result<Self> {
        repo.seed_catalog_membership_from_records()?;
        for summary in repo.list_cataloged_local_repo_summaries()? {
            RemoteImportService::recover_startup(&repo, summary.repo_id)?;
        }
        let sync = Arc::new(SyncManager::new_checked(repo.clone())?);
        sync.scan()?;
        let (tx, _rx) = broadcast::channel::<ServerMessage>(32);
        let watchers = Arc::new(start_file_watchers(sync.clone(), tx.clone())?);
        let watcher_view = watchers.view();
        let membership = repo.catalog_membership_runtime();
        let remote_import = Arc::new(RemoteImportCoordinator::new(
            repo.clone(),
            sync.clone(),
            membership.clone(),
        ));
        let gate = Arc::new(RepoMutationPublicationGate::new(
            watcher_view.clone(),
            repo.claim_repo_catalog_cut_authority()?,
        ));
        let coordinator = RepoLifecycleCoordinator::new(
            repo.clone(),
            sync.clone(),
            gate,
            watchers.clone(),
            remote_import.clone(),
            membership.clone(),
            None,
        );
        let jobs = RepoLifecycleJobRuntime::start(
            repo.ledger_dir(),
            Arc::new(RepoLifecycleHostExecutor::new(
                coordinator,
                repo.clone(),
                watcher_view.clone(),
                sync,
                remote_import,
            )),
            Arc::new(RepoLifecycleHostPublicationSink::new(
                repo.clone(),
                watcher_view,
                RepoSessionRuntime::new(membership),
                tx,
            )),
        )?;
        Ok(Self {
            repo,
            jobs,
            watchers,
        })
    }

    pub(crate) async fn prepare(&self, repo_id: RepoId) -> Result<OfflineRemovalPrepared> {
        let prepared = self
            .jobs
            .prepare_removal(RepoRemovalPrepareIntent {
                request_id: Uuid::new_v4(),
                repo_id,
                scope_nonce: SCOPE_NONCE,
                fallback_repo_id: None,
                issuer: self.offline_issuer(repo_id)?,
            })
            .await?;
        Ok(OfflineRemovalPrepared {
            preparation_id: prepared.preparation_id,
            repo_id: prepared.repo_id,
            preview: prepared.preview,
            confirmation_token: prepared.confirmation_token,
        })
    }

    pub(crate) async fn execute(
        &self,
        repo_id: RepoId,
        preparation_id: Uuid,
        execute_request_id: Uuid,
        confirmation_token: RemovalConfirmationToken,
    ) -> Result<OfflineRemovalAccepted> {
        let accepted = self
            .jobs
            .execute_removal(RepoRemovalExecuteIntent {
                request_id: execute_request_id,
                expected_repo_id: Some(repo_id),
                preparation_id,
                confirmation_token,
                fallback_binding: None,
                scope_nonce: SCOPE_NONCE,
                switch_nonce: SWITCH_NONCE,
                issuer: self.offline_issuer(repo_id)?,
            })
            .await?;
        Ok(OfflineRemovalAccepted {
            request_id: accepted.request_id,
            job_id: accepted.job_id,
            repo_id: accepted.target_repo_id,
        })
    }

    pub(crate) async fn status_if_known(
        &self,
        request_id: Uuid,
    ) -> Result<Option<OfflineRemovalStatus>> {
        match self.jobs.status(request_id).await {
            Ok(status) => Ok(Some(OfflineRemovalStatus {
                request_id: status.request_id,
                repo_id: status.target_repo_id,
                state: map_phase(status.phase),
                outcome: status.outcome.map(map_outcome),
                publication_pending: status.publication_pending,
            })),
            Err(RepoLifecycleJobError::NotFound) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn shutdown(self) -> Result<()> {
        let lifecycle = self.jobs.shutdown().await.map_err(anyhow::Error::from);
        let watchers = tokio::task::spawn_blocking(move || self.watchers.shutdown())
            .await
            .map_err(|error| anyhow::anyhow!("watcher shutdown task failed: {error}"))?
            .map_err(anyhow::Error::from);
        lifecycle?;
        watchers
    }

    fn offline_issuer(&self, repo_id: RepoId) -> Result<RepoRemovalIssuerBinding> {
        let authority_root =
            HostPathIdentity::capture(self.repo.ledger_dir(), HostPathKind::Directory)?;
        let authority_lock = self
            .repo
            .snapshot_local_authority_for_removal(repo_id)?
            .authority_lock()
            .clone();
        Ok(RepoRemovalIssuerBinding::OfflineAuthority {
            authority_root,
            authority_lock,
        })
    }
}

fn map_phase(phase: RepoLifecycleJobPhase) -> RepoLifecycleState {
    match phase {
        RepoLifecycleJobPhase::Accepted => RepoLifecycleState::Accepted,
        RepoLifecycleJobPhase::Running => RepoLifecycleState::Running,
        RepoLifecycleJobPhase::Recovering => RepoLifecycleState::Recovering,
        RepoLifecycleJobPhase::Terminal => RepoLifecycleState::Terminal,
    }
}

fn map_outcome(outcome: RepoLifecycleJobOutcome) -> RepoLifecycleOutcome {
    match outcome {
        RepoLifecycleJobOutcome::Succeeded => RepoLifecycleOutcome::Succeeded,
        RepoLifecycleJobOutcome::NotCommitted => RepoLifecycleOutcome::NotCommitted,
        RepoLifecycleJobOutcome::CommittedPartial => RepoLifecycleOutcome::CommittedPartial,
        RepoLifecycleJobOutcome::RepairRequired => RepoLifecycleOutcome::RepairRequired,
    }
}
