//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-provider-dispatch-contract
//!   - 06_backup#projection-backup-upload-state-machine-contract
//!   - 06_backup#projection-backup-pull-state-machine-contract
//!   - 06_backup#projection-backup-command-output-contract
//!   - 06_backup#projection-backup-verification-contract
//!
//! Resolved-repo Remote Projection provider execution used by server-side intents.

use super::{
    collect, outcome_contract, provider_io_not_ready, rollback_after_failed_scan, s3, webdav,
    workspace_apply,
};
use crate::commands::source_control_workspace_gate::ensure_local_repo_workspace_identity_for_write;
use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
#[cfg(test)]
use deve_core::remote_projection::RemoteProjectionFile;
use deve_core::remote_projection::{
    RemoteProjectionDirection, RemoteProjectionPlanInput, RemoteProjectionProvider,
    RemoteProjectionPullOutcome, plan_remote_projection_transport,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionRemoteExecutionSummary {
    pub(crate) provider: RemoteProjectionProvider,
    pub(crate) direction: RemoteProjectionDirection,
    pub(crate) provider_io_ready: bool,
    pub(crate) uploaded_files: usize,
    pub(crate) downloaded_files: usize,
    pub(crate) external_changes_scan_triggered: bool,
}

/// Provider output captured before the repository mutation permit is acquired.
/// It contains no local workspace or authority side effects.
pub(crate) struct PreparedProjectionRemotePull {
    provider: RemoteProjectionProvider,
    outcome: RemoteProjectionPullOutcome,
}

/// Applied workspace files whose External Changes scan still has to finish.
/// Dropping this value before `finish_prepared_pull` rolls the workspace apply
/// back through `AppliedPullFiles`' cleanup guard.
pub(crate) struct AppliedProjectionRemotePull {
    provider: RemoteProjectionProvider,
    downloaded_files: usize,
    applied: workspace_apply::AppliedPullFiles,
}

impl AppliedProjectionRemotePull {
    pub(crate) fn defer_rollback(mut self) -> Self {
        self.applied = self.applied.defer_rollback();
        self
    }
}

pub(crate) fn run_for_resolved_repo(
    repo: Arc<RepoManager>,
    repo_name: &str,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    locator: &str,
) -> Result<ProjectionRemoteExecutionSummary> {
    let mut webdav_provider = webdav::WebDavProjectionProvider::new()?;
    if provider == RemoteProjectionProvider::S3 {
        let mut s3_provider = s3::S3ProjectionProvider::new()?;
        run_for_resolved_repo_with_providers(
            repo,
            repo_name,
            provider,
            direction,
            locator,
            &mut webdav_provider,
            &mut s3_provider,
        )
    } else {
        let mut s3_provider = s3::FailClosedS3ProjectionProvider;
        run_for_resolved_repo_with_providers(
            repo,
            repo_name,
            provider,
            direction,
            locator,
            &mut webdav_provider,
            &mut s3_provider,
        )
    }
}

fn run_for_resolved_repo_with_providers(
    repo: Arc<RepoManager>,
    repo_name: &str,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    locator: &str,
    webdav_provider: &mut dyn webdav::WebDavProjectionAdapter,
    s3_provider: &mut dyn s3::S3ProjectionAdapter,
) -> Result<ProjectionRemoteExecutionSummary> {
    let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
        provider,
        direction,
        locator: locator.to_string(),
    })?;
    match direction {
        RemoteProjectionDirection::Push => {
            ensure_local_repo_workspace_identity_for_write(
                repo.as_ref(),
                repo_name,
                "remote projection transport",
            )?;
            let workspace = repo.local_repo_workspace_root(repo_name)?;
            let files = collect::collect_markdown_projection_files(&workspace)?;
            let outcome = match provider {
                RemoteProjectionProvider::WebDav => {
                    webdav_provider.push_projection_files(provider, &plan.locator, &files)
                }
                RemoteProjectionProvider::S3 => {
                    s3_provider.push_projection_files(provider, &plan.locator, &files)
                }
            }
            .map_err(provider_io_not_ready)?;
            outcome_contract::ensure_projection_transport_push_outcome_contract(&outcome)?;
            Ok(ProjectionRemoteExecutionSummary {
                provider,
                direction,
                provider_io_ready: true,
                uploaded_files: outcome.uploaded_files,
                downloaded_files: 0,
                external_changes_scan_triggered: false,
            })
        }
        RemoteProjectionDirection::Pull => {
            let prepared =
                prepare_pull_with_providers(provider, &plan.locator, webdav_provider, s3_provider)?;
            let applied = apply_prepared_pull(repo.clone(), repo_name, prepared)?;
            finish_prepared_pull(repo, repo_name, applied)
        }
    }
}

pub(crate) fn prepare_pull_for_resolved_repo(
    provider: RemoteProjectionProvider,
    locator: &str,
) -> Result<PreparedProjectionRemotePull> {
    let mut webdav_provider = webdav::WebDavProjectionProvider::new()?;
    if provider == RemoteProjectionProvider::S3 {
        let mut s3_provider = s3::S3ProjectionProvider::new()?;
        prepare_pull_with_providers(provider, locator, &mut webdav_provider, &mut s3_provider)
    } else {
        let mut s3_provider = s3::FailClosedS3ProjectionProvider;
        prepare_pull_with_providers(provider, locator, &mut webdav_provider, &mut s3_provider)
    }
}

fn prepare_pull_with_providers(
    provider: RemoteProjectionProvider,
    locator: &str,
    webdav_provider: &mut dyn webdav::WebDavProjectionAdapter,
    s3_provider: &mut dyn s3::S3ProjectionAdapter,
) -> Result<PreparedProjectionRemotePull> {
    let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
        provider,
        direction: RemoteProjectionDirection::Pull,
        locator: locator.to_string(),
    })?;
    let outcome = match provider {
        RemoteProjectionProvider::WebDav => {
            webdav_provider.pull_projection_files(provider, &plan.locator)
        }
        RemoteProjectionProvider::S3 => s3_provider.pull_projection_files(provider, &plan.locator),
    }
    .map_err(provider_io_not_ready)?;
    outcome_contract::ensure_projection_transport_pull_outcome_contract(&outcome)?;
    Ok(PreparedProjectionRemotePull { provider, outcome })
}

pub(crate) fn apply_prepared_pull(
    repo: Arc<RepoManager>,
    repo_name: &str,
    prepared: PreparedProjectionRemotePull,
) -> Result<AppliedProjectionRemotePull> {
    ensure_local_repo_workspace_identity_for_write(
        repo.as_ref(),
        repo_name,
        "remote projection transport",
    )?;
    let workspace = repo.local_repo_workspace_root(repo_name)?;
    let downloaded_files = prepared.outcome.files.len();
    let applied = workspace_apply::write_pull_files(&workspace, &prepared.outcome.files)?;
    Ok(AppliedProjectionRemotePull {
        provider: prepared.provider,
        downloaded_files,
        applied,
    })
}

pub(crate) fn finish_prepared_pull(
    repo: Arc<RepoManager>,
    repo_name: &str,
    applied: AppliedProjectionRemotePull,
) -> Result<ProjectionRemoteExecutionSummary> {
    let AppliedProjectionRemotePull {
        provider,
        downloaded_files,
        applied,
    } = applied;
    let sync_manager = match deve_core::sync::SyncManager::new_checked(repo) {
        Ok(sync_manager) => sync_manager,
        Err(err) => {
            rollback_after_failed_scan(applied, &err)?;
            return Err(err);
        }
    };
    if let Err(err) = sync_manager.scan_repo(repo_name) {
        rollback_after_failed_scan(applied, &err)?;
        return Err(err);
    }
    applied.commit();
    Ok(ProjectionRemoteExecutionSummary {
        provider,
        direction: RemoteProjectionDirection::Pull,
        provider_io_ready: true,
        uploaded_files: 0,
        downloaded_files,
        external_changes_scan_triggered: true,
    })
}

pub(crate) fn scan_prepared_pull(repo: Arc<RepoManager>, repo_name: &str) -> Result<()> {
    deve_core::sync::SyncManager::new_checked(repo)?.scan_repo(repo_name)
}

pub(crate) fn finalize_prepared_pull_after_scan(
    applied: AppliedProjectionRemotePull,
    scan_result: Result<()>,
) -> Result<ProjectionRemoteExecutionSummary> {
    let AppliedProjectionRemotePull {
        provider,
        downloaded_files,
        applied,
    } = applied;
    if let Err(scan_error) = scan_result {
        applied
            .rollback_after_failed_scan_if_unchanged()
            .with_context(|| {
                format!("remote projection pull scan failed after workspace apply: {scan_error}")
            })?;
        return Err(scan_error);
    }
    applied.commit();
    Ok(ProjectionRemoteExecutionSummary {
        provider,
        direction: RemoteProjectionDirection::Pull,
        provider_io_ready: true,
        uploaded_files: 0,
        downloaded_files,
        external_changes_scan_triggered: true,
    })
}

#[cfg(test)]
pub(crate) fn prepared_pull_for_test(
    provider: RemoteProjectionProvider,
    files: Vec<RemoteProjectionFile>,
) -> PreparedProjectionRemotePull {
    PreparedProjectionRemotePull {
        provider,
        outcome: RemoteProjectionPullOutcome {
            files,
            effects:
                deve_core::remote_projection::RemoteProjectionAuthorityEffects::projection_transport(
                ),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        },
    }
}
