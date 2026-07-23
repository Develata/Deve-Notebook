//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#repo-removal-command-contract
//!
//! CLI shell for backend-owned Prepare/Execute removal. Both offline and
//! server-owned paths use the same lifecycle runtime and receipt store.

mod direct;
mod output;
mod proxy;
mod token;

use crate::commands::live_proxy;
use crate::commands::remote_import::LocalCliAuthArgs;
use crate::server::RepoLifecycleJobError;
use crate::server::local_repo_removal_cli_runtime::OfflineRemovalRuntime;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) async fn run(
    ledger_dir: &Path,
    repo_id: RepoId,
    apply: bool,
    token: Option<&str>,
    auth: LocalCliAuthArgs,
    snapshot_depth: usize,
) -> Result<()> {
    run_inner(ledger_dir, repo_id, apply, token, auth, snapshot_depth)
        .await
        .map_err(output::sanitize)
}

async fn run_inner(
    ledger_dir: &Path,
    repo_id: RepoId,
    apply: bool,
    token: Option<&str>,
    auth: LocalCliAuthArgs,
    snapshot_depth: usize,
) -> Result<()> {
    if apply != token.is_some() {
        bail!("--apply and --token must be supplied together");
    }
    match RepoManager::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id) {
        Ok(repo) => direct::run(Arc::new(repo), repo_id, token).await,
        Err(error) if live_proxy::is_db_lock_error(&error) => {
            proxy::run(ledger_dir, repo_id, token, auth).await
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn run_repair(
    ledger_dir: &Path,
    request_id: Uuid,
    apply: bool,
    token: Option<&str>,
    auth: LocalCliAuthArgs,
    snapshot_depth: usize,
) -> Result<()> {
    run_repair_inner(ledger_dir, request_id, apply, token, auth, snapshot_depth)
        .await
        .map_err(output::sanitize)
}

async fn run_repair_inner(
    ledger_dir: &Path,
    request_id: Uuid,
    apply: bool,
    token: Option<&str>,
    auth: LocalCliAuthArgs,
    snapshot_depth: usize,
) -> Result<()> {
    if request_id.is_nil() {
        bail!("REPO_LIFECYCLE_INVALID_REQUEST");
    }
    if apply != token.is_some() {
        bail!("--apply and --token must be supplied together");
    }
    let canonical_ledger_dir = std::fs::canonicalize(ledger_dir)?;
    let claim = match OfflineRemovalRuntime::claim_repair(&canonical_ledger_dir) {
        Ok(claim) => claim,
        Err(RepoLifecycleJobError::OwnerActive) => {
            return proxy::run_repair(&canonical_ledger_dir, request_id, token, auth).await;
        }
        Err(error) => return Err(error.into()),
    };
    let repo = Arc::new(RepoManager::init_empty_host(
        &canonical_ledger_dir,
        snapshot_depth,
    )?);
    direct::run_repair(repo, claim, request_id, token).await
}
