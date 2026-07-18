//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-upload-state-machine-contract
//!   - 14_commands#cli-commands
//!
//! Push-only CLI shell for remote Markdown projection transport intents.

mod profile_command;

use crate::commands::repo_arg::resolve_local_repo_args;
use crate::remote_projection_transport::{self, ProjectionPushError, ProjectionPushSource};
use crate::remote_projection_transport::{s3, webdav};
use crate::workspace_identity_gate::ensure_local_repo_workspace_identity_for_write;
use anyhow::{Result, bail};
use clap::Subcommand;
use deve_core::ledger::RepoManager;
use deve_core::remote_projection::{
    RemoteProjectionProvider, RemoteProjectionPushOutcome, validate_remote_projection_locator,
};
use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
};
use std::path::Path;
use std::sync::Arc;

pub(crate) use profile_command::run_s3_profile_action;

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectionRemoteAction {
    /// Push through a WebDAV projection transport
    Webdav {
        #[command(subcommand)]
        action: ProjectionRemoteDirectionAction,
    },
    /// Push through an S3 projection transport or manage S3 profiles
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

    let explicit_s3_profile = s3_profile_id_from_action(&action).map(str::to_owned);
    let request = request_from_action(action);
    match request.provider {
        RemoteProjectionProvider::WebDav => {
            let mut provider = webdav::WebDavProjectionProvider::new()?;
            run_with_push_provider(
                ledger_dir,
                request,
                snapshot_depth,
                &mut WebDavPushProvider(&mut provider),
            )
        }
        RemoteProjectionProvider::S3 => {
            let mut provider = if let Some(profile_id) = explicit_s3_profile {
                let profile = s3::load_remote_projection_s3_profile(ledger_dir, &profile_id)?;
                s3::S3ProjectionProvider::new()?.with_custom_profile(profile)
            } else {
                s3::provider_for_locator(
                    ledger_dir,
                    remote_projection_transport::TransportCapability::Push,
                    &request.locator,
                )?
                .0
            };
            run_with_push_provider(
                ledger_dir,
                request,
                snapshot_depth,
                &mut S3PushProvider(&mut provider),
            )
        }
    }
}

trait PushProvider {
    fn push(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> std::result::Result<RemoteProjectionPushOutcome, ProjectionPushError>;
}

struct WebDavPushProvider<'a>(&'a mut dyn webdav::WebDavProjectionPushAdapter);

impl PushProvider for WebDavPushProvider<'_> {
    fn push(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> std::result::Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        self.0.push_projection_files(provider, locator, source)
    }
}

struct S3PushProvider<'a>(&'a mut dyn s3::S3ProjectionPushAdapter);

impl PushProvider for S3PushProvider<'_> {
    fn push(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> std::result::Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        self.0.push_projection_files(provider, locator, source)
    }
}

#[cfg(test)]
fn run_with_provider(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
    provider: &mut dyn webdav::WebDavProjectionPushAdapter,
) -> Result<()> {
    let request = request_from_action(action);
    run_with_push_provider(
        ledger_dir,
        request,
        snapshot_depth,
        &mut WebDavPushProvider(provider),
    )
}

#[cfg(test)]
fn run_with_providers(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
    webdav_provider: &mut dyn webdav::WebDavProjectionPushAdapter,
    s3_provider: &mut dyn s3::S3ProjectionPushAdapter,
) -> Result<()> {
    let request = request_from_action(action);
    match request.provider {
        RemoteProjectionProvider::WebDav => run_with_push_provider(
            ledger_dir,
            request,
            snapshot_depth,
            &mut WebDavPushProvider(webdav_provider),
        ),
        RemoteProjectionProvider::S3 => run_with_push_provider(
            ledger_dir,
            request,
            snapshot_depth,
            &mut S3PushProvider(s3_provider),
        ),
    }
}

fn run_with_push_provider(
    ledger_dir: &Path,
    request: ProjectionRemoteRequest,
    snapshot_depth: usize,
    provider: &mut dyn PushProvider,
) -> Result<()> {
    let locator = validate_remote_projection_locator(request.provider, &request.locator)?;
    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth, None, None)?);
    let repo_names = resolve_local_repo_args(repo.as_ref(), request.repo.as_deref())?;
    if request.repo.is_none() && repo_names.len() != 1 {
        bail!(
            "remote projection provider I/O requires an explicit --repo when multiple local repos are present"
        );
    }

    for repo_name in repo_names {
        ensure_local_repo_workspace_identity_for_write(
            repo.as_ref(),
            &repo_name,
            "remote projection push",
        )?;
        let workspace = repo.local_repo_workspace_root(&repo_name)?;
        let watcher = start_direct_push_watcher(repo.clone(), &repo_name)?;
        let push_result = (|| {
            let source =
                remote_projection_transport::WorkspaceProjectionPushSource::collect(&workspace)?;
            let outcome = provider.push(request.provider, &locator, &source)?;
            remote_projection_transport::ensure_projection_transport_push_outcome_contract(
                &outcome,
            )?;
            println!(
                "projection_remote[{repo_name}]: provider={} direction=push scope=markdown workspace={} writes_ledger=false provider_io_ready=true uploaded_files={} writes_source_control_staging={} writes_commit_anchor={} writes_git_main_mirror={} provider_metadata_diagnostic_only={}",
                request.provider.as_str(),
                workspace.display(),
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
    Ok(())
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

#[derive(Debug, PartialEq, Eq)]
struct ProjectionRemoteRequest {
    provider: RemoteProjectionProvider,
    repo: Option<String>,
    locator: String,
}

fn request_from_action(action: ProjectionRemoteAction) -> ProjectionRemoteRequest {
    match action {
        ProjectionRemoteAction::Webdav {
            action: ProjectionRemoteDirectionAction::Push { repo, locator },
        } => ProjectionRemoteRequest {
            provider: RemoteProjectionProvider::WebDav,
            repo,
            locator,
        },
        ProjectionRemoteAction::S3 {
            action:
                S3ProjectionRemoteAction::Push {
                    repo,
                    locator,
                    profile: _,
                },
        } => ProjectionRemoteRequest {
            provider: RemoteProjectionProvider::S3,
            repo,
            locator,
        },
        ProjectionRemoteAction::S3 {
            action: S3ProjectionRemoteAction::Profile { .. },
        } => unreachable!("S3 profile management is handled before provider execution"),
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
        } => Some(profile.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
