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
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::remote_projection::{
    RemoteProjectionDirection, RemoteProjectionPlanInput, RemoteProjectionProvider,
    plan_remote_projection_transport,
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
    ensure_local_repo_workspace_identity_for_write(
        repo.as_ref(),
        repo_name,
        "remote projection transport",
    )?;
    let workspace = repo.local_repo_workspace_root(repo_name)?;
    match direction {
        RemoteProjectionDirection::Push => {
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
            let outcome = match provider {
                RemoteProjectionProvider::WebDav => {
                    webdav_provider.pull_projection_files(provider, &plan.locator)
                }
                RemoteProjectionProvider::S3 => {
                    s3_provider.pull_projection_files(provider, &plan.locator)
                }
            }
            .map_err(provider_io_not_ready)?;
            outcome_contract::ensure_projection_transport_pull_outcome_contract(&outcome)?;
            let downloaded_files = outcome.files.len();
            let applied = workspace_apply::write_pull_files(&workspace, &outcome.files)?;
            let sync_manager = match deve_core::sync::SyncManager::new_checked(repo.clone()) {
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
                direction,
                provider_io_ready: true,
                uploaded_files: 0,
                downloaded_files,
                external_changes_scan_triggered: true,
            })
        }
    }
}
