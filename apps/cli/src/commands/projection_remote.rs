//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-upload-state-machine-contract
//!   - 14_commands#cli-commands
//!
//! CLI shell for remote Markdown projection transport intents.

mod profile_command;

use crate::commands::repo_arg::resolve_local_repo_args;
use crate::remote_projection_legacy;
use crate::remote_projection_transport::{self, ProjectionPushSource};
use crate::remote_projection_transport::{s3, webdav};
use crate::workspace_identity_gate::ensure_local_repo_workspace_identity_for_write;
use anyhow::{Result, bail};
use clap::Subcommand;
use deve_core::ledger::RepoManager;
use deve_core::remote_projection::{
    RemoteProjectionDirection, RemoteProjectionPlanInput, RemoteProjectionProvider,
    plan_remote_projection_transport,
};
use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
};
use std::path::Path;
use std::sync::Arc;

pub(crate) use profile_command::run_s3_profile_action;

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectionRemoteAction {
    /// Push or pull through a WebDAV projection transport
    Webdav {
        #[command(subcommand)]
        action: ProjectionRemoteDirectionAction,
    },
    /// Push or pull through an S3 projection transport
    S3 {
        #[command(subcommand)]
        action: S3ProjectionRemoteAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectionRemoteDirectionAction {
    /// Upload the Markdown projection folder
    Push {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
    },
    /// Download and overwrite the Markdown projection folder
    Pull {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum S3ProjectionRemoteAction {
    /// Upload the Markdown projection folder
    Push {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Download and overwrite the Markdown projection folder
    Pull {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Manage host-local secret-free S3-compatible Remote Projection profiles
    Profile {
        #[command(subcommand)]
        action: S3ProjectionProfileAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum S3ProjectionProfileAction {
    /// Write or replace a host-local S3-compatible Remote Projection profile
    Put {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        endpoint_origin: String,
        #[arg(long)]
        bucket: String,
        #[arg(long)]
        allowed_prefix: String,
        #[arg(long)]
        region: String,
        #[arg(long)]
        credential_env_prefix: String,
        #[arg(long, value_delimiter = ',', default_value = "push,source-acquisition")]
        allowed_capabilities: Vec<String>,
    },
    /// List host-local S3-compatible Remote Projection profile handles
    List,
}

pub(crate) fn run(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
) -> Result<()> {
    if let ProjectionRemoteAction::S3 {
        action: S3ProjectionRemoteAction::Profile { action },
    } = &action
    {
        return run_s3_profile_action(ledger_dir, action);
    }
    let mut webdav_provider = webdav::WebDavProjectionProvider::new()?;
    if action_is_s3(&action) {
        let mut s3_provider = s3::S3ProjectionProvider::new()?;
        if let Some(profile_id) = s3_profile_id_from_action(&action) {
            let profile = s3::load_remote_projection_s3_profile(ledger_dir, profile_id)?;
            s3_provider = s3_provider.with_custom_profile(profile);
        }
        run_with_providers(
            ledger_dir,
            action,
            snapshot_depth,
            &mut webdav_provider,
            &mut s3_provider,
        )
    } else {
        let mut s3_provider = s3::FailClosedS3ProjectionProvider;
        run_with_providers(
            ledger_dir,
            action,
            snapshot_depth,
            &mut webdav_provider,
            &mut s3_provider,
        )
    }
}

#[cfg(test)]
fn run_with_provider(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
    webdav_provider: &mut dyn remote_projection_legacy::LegacyWebDavProjectionAdapter,
) -> Result<()> {
    let mut s3_provider = s3::FailClosedS3ProjectionProvider;
    run_with_providers(
        ledger_dir,
        action,
        snapshot_depth,
        webdav_provider,
        &mut s3_provider,
    )
}

fn run_with_providers(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
    webdav_provider: &mut dyn remote_projection_legacy::LegacyWebDavProjectionAdapter,
    s3_provider: &mut dyn remote_projection_legacy::LegacyS3ProjectionAdapter,
) -> Result<()> {
    let request = request_from_action(action);
    let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
        provider: request.provider,
        direction: request.direction,
        locator: request.locator.clone(),
    })?;
    let provider_direction_wired = provider_direction_wired_for(&request);

    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth, None, None)?);
    let repo_names = resolve_local_repo_args(repo.as_ref(), request.repo.as_deref())?;
    if provider_direction_wired && request.repo.is_none() && repo_names.len() != 1 {
        bail!(
            "remote projection provider I/O requires an explicit --repo when multiple local repos are present"
        );
    }
    for repo_name in repo_names {
        ensure_local_repo_workspace_identity_for_write(
            repo.as_ref(),
            &repo_name,
            "remote projection transport",
        )?;
        let workspace = repo.local_repo_workspace_root(&repo_name)?;
        if provider_direction_wired {
            match request.direction {
                RemoteProjectionDirection::Push => {
                    let watcher = start_direct_push_watcher(repo.clone(), &repo_name)?;
                    let push_result = (|| {
                        let source =
                            remote_projection_transport::WorkspaceProjectionPushSource::collect(
                                &workspace,
                            )?;
                        println!(
                            "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_direction_wired=true provider_io_state=pending planned_files={}",
                            plan.provider.as_str(),
                            plan.direction.as_str(),
                            plan.projection_scope,
                            workspace.display(),
                            plan.writes_ledger,
                            plan.external_changes_confirmation_required,
                            source.file_count(),
                        );
                        let outcome = match request.provider {
                            RemoteProjectionProvider::WebDav => webdav_provider
                                .push_projection_files(request.provider, &plan.locator, &source),
                            RemoteProjectionProvider::S3 => s3_provider.push_projection_files(
                                request.provider,
                                &plan.locator,
                                &source,
                            ),
                        }
                        .map_err(remote_projection_transport::provider_io_not_ready)?;
                        remote_projection_transport::ensure_projection_transport_push_outcome_contract(
                            &outcome,
                        )?;
                        println!(
                            "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready=true uploaded_files={} writes_source_control_staging={} writes_commit_anchor={} writes_git_main_mirror={} provider_metadata_diagnostic_only={}",
                            plan.provider.as_str(),
                            plan.direction.as_str(),
                            plan.projection_scope,
                            workspace.display(),
                            plan.writes_ledger,
                            plan.external_changes_confirmation_required,
                            outcome.uploaded_files,
                            outcome.effects.writes_source_control_staging,
                            outcome.effects.writes_commit_anchor,
                            outcome.effects.writes_git_main_mirror,
                            outcome.provider_metadata_is_diagnostic_only,
                        );
                        Ok(())
                    })();
                    finish_direct_push(push_result, watcher.shutdown())?;
                }
                RemoteProjectionDirection::Pull => {
                    println!(
                        "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_direction_wired=true provider_io_state=pending",
                        plan.provider.as_str(),
                        plan.direction.as_str(),
                        plan.projection_scope,
                        workspace.display(),
                        plan.writes_ledger,
                        plan.external_changes_confirmation_required,
                    );
                    let outcome = match request.provider {
                        RemoteProjectionProvider::WebDav => {
                            webdav_provider.pull_projection_files(request.provider, &plan.locator)
                        }
                        RemoteProjectionProvider::S3 => {
                            s3_provider.pull_projection_files(request.provider, &plan.locator)
                        }
                    }
                    .map_err(remote_projection_transport::provider_io_not_ready)?;
                    remote_projection_legacy::ensure_projection_transport_pull_outcome_contract(
                        &outcome,
                    )?;
                    let applied =
                        remote_projection_legacy::write_pull_files(&workspace, &outcome.files)?;
                    let sync_manager = match deve_core::sync::SyncManager::new_checked(repo.clone())
                    {
                        Ok(sync_manager) => sync_manager,
                        Err(err) => {
                            remote_projection_legacy::rollback_after_failed_scan(applied, &err)?;
                            return Err(err);
                        }
                    };
                    if let Err(err) = sync_manager.scan_repo(&repo_name) {
                        remote_projection_legacy::rollback_after_failed_scan(applied, &err)?;
                        return Err(err);
                    }
                    applied.commit();
                    println!(
                        "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready=true downloaded_files={} overwrites_projection_workspace={} writes_source_control_staging={} writes_commit_anchor={} writes_git_main_mirror={} provider_metadata_diagnostic_only={} external_changes_scan_triggered=true",
                        plan.provider.as_str(),
                        plan.direction.as_str(),
                        plan.projection_scope,
                        workspace.display(),
                        plan.writes_ledger,
                        plan.external_changes_confirmation_required,
                        outcome.files.len(),
                        outcome.overwrites_projection_workspace,
                        outcome.effects.writes_source_control_staging,
                        outcome.effects.writes_commit_anchor,
                        outcome.effects.writes_git_main_mirror,
                        outcome.provider_metadata_is_diagnostic_only,
                    );
                }
            }
        } else {
            println!(
                "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready=false",
                plan.provider.as_str(),
                plan.direction.as_str(),
                plan.projection_scope,
                workspace.display(),
                plan.writes_ledger,
                plan.external_changes_confirmation_required,
            );
        }
    }
    if provider_direction_wired {
        return Ok(());
    }
    bail!(
        "remote projection provider I/O is not wired yet (provider_io_ready=false); no projection files were pushed or pulled"
    );
}

fn start_direct_push_watcher(repo: Arc<RepoManager>, repo_name: &str) -> Result<RepoWatcherHandle> {
    let sync = Arc::new(deve_core::sync::SyncManager::new_checked(repo)?);
    if !sync
        .healthy_local_repo_names_for_execution()?
        .iter()
        .any(|healthy| healthy == repo_name)
    {
        bail!("remote projection push requires a Healthy local repository");
    }
    let handle = RepoWatcherHandle::start(RepoWatcherStart::resolve(sync, repo_name, 1)?)?;
    let failure = match handle.snapshot().worker_state() {
        RepoWatcherWorkerState::Running => return Ok(handle),
        RepoWatcherWorkerState::Failed(failure) => failure.clone(),
    };
    match handle.shutdown() {
        Ok(()) => Err(anyhow::Error::new(failure)),
        Err(cleanup) => Err(anyhow::Error::new(failure)
            .context(format!("temporary watcher shutdown also failed: {cleanup}"))),
    }
}

fn finish_direct_push(
    push_result: Result<()>,
    shutdown_result: Result<(), WatcherFailure>,
) -> Result<()> {
    match (push_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(shutdown)) => Err(anyhow::Error::new(shutdown)),
        (Err(primary), Err(shutdown)) => Err(primary.context(format!(
            "temporary watcher shutdown also failed: {shutdown}"
        ))),
    }
}

struct ProjectionRemoteRequest {
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    repo: Option<String>,
    locator: String,
}

fn request_from_action(action: ProjectionRemoteAction) -> ProjectionRemoteRequest {
    match action {
        ProjectionRemoteAction::Webdav { action } => {
            direction_request(RemoteProjectionProvider::WebDav, action)
        }
        ProjectionRemoteAction::S3 { action } => s3_direction_request(action),
    }
}

fn s3_direction_request(action: S3ProjectionRemoteAction) -> ProjectionRemoteRequest {
    match action {
        S3ProjectionRemoteAction::Push {
            repo,
            locator,
            profile: _,
        } => ProjectionRemoteRequest {
            provider: RemoteProjectionProvider::S3,
            direction: RemoteProjectionDirection::Push,
            repo,
            locator,
        },
        S3ProjectionRemoteAction::Pull {
            repo,
            locator,
            profile: _,
        } => ProjectionRemoteRequest {
            provider: RemoteProjectionProvider::S3,
            direction: RemoteProjectionDirection::Pull,
            repo,
            locator,
        },
        S3ProjectionRemoteAction::Profile { .. } => {
            unreachable!("S3 profile management actions are handled before provider execution")
        }
    }
}

fn s3_profile_id_from_action(action: &ProjectionRemoteAction) -> Option<&str> {
    match action {
        ProjectionRemoteAction::S3 {
            action:
                S3ProjectionRemoteAction::Push {
                    profile: Some(profile),
                    ..
                },
        }
        | ProjectionRemoteAction::S3 {
            action:
                S3ProjectionRemoteAction::Pull {
                    profile: Some(profile),
                    ..
                },
        } => Some(profile.as_str()),
        _ => None,
    }
}

fn direction_request(
    provider: RemoteProjectionProvider,
    action: ProjectionRemoteDirectionAction,
) -> ProjectionRemoteRequest {
    match action {
        ProjectionRemoteDirectionAction::Push { repo, locator } => ProjectionRemoteRequest {
            provider,
            direction: RemoteProjectionDirection::Push,
            repo,
            locator,
        },
        ProjectionRemoteDirectionAction::Pull { repo, locator } => ProjectionRemoteRequest {
            provider,
            direction: RemoteProjectionDirection::Pull,
            repo,
            locator,
        },
    }
}

fn action_is_s3(action: &ProjectionRemoteAction) -> bool {
    matches!(action, ProjectionRemoteAction::S3 { .. })
}

fn provider_direction_wired_for(request: &ProjectionRemoteRequest) -> bool {
    matches!(
        (request.provider, request.direction),
        (
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Push
        ) | (
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull
        ) | (
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Push
        ) | (
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Pull
        )
    )
}

#[cfg(test)]
mod tests;
