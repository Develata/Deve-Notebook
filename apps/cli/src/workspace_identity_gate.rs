//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 06_backup#remote-projection-transport-contract
//!   - 14_commands#cli-commands
//!
//! Shared host gate for local workspace-dependent operations.

use anyhow::{Result, anyhow};
use deve_core::ledger::RepoManager;

pub(crate) fn ensure_local_repo_workspace_identity_for_write(
    repo: &RepoManager,
    repo_name: &str,
    action: &str,
) -> Result<()> {
    repo.check_projection_locator_for_local_repo(repo_name)
        .map(|_| ())
        .map_err(|err| {
            anyhow!(
                "Local repo Projection workspace identity marker is invalid; repair before {action}: {repo_name}: {err}"
            )
        })
}
