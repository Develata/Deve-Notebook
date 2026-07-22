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
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use std::path::Path;
use std::sync::Arc;

pub(crate) async fn run(
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
