//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Shared mounted admission for local Source Control mutations.

use super::errors::{ScOp, map_repo_error};
use super::service::ScResult;
use crate::server::AppState;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::RepoId;
use std::sync::Arc;

pub(super) async fn execute<T>(
    state: &Arc<AppState>,
    repo_id: RepoId,
    expected_name: &str,
    rebind_op: ScOp,
    operation: impl FnOnce(&RepoSelector) -> ScResult<T>,
) -> ScResult<T> {
    let expected_name = match crate::server::repo_mutation::prepare_writable_local_repo(
        state,
        repo_id,
        expected_name,
    ) {
        Ok(name) => name,
        Err(error) => return Err(map_repo_error(rebind_op, error)),
    };
    let execution = state
        .repo_mutation_gate()
        .execute_mounted_repo_unpublished(repo_id, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                state,
                repo_id,
                &expected_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => return Err(map_repo_error(rebind_op, error)),
            };
            let selector = RepoSelector {
                repo_id: Some(repo_id),
                repo_name: Some(repo_name),
            };
            operation(&selector)
        })
        .await;
    match execution {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(error.server_error()),
    }
}
