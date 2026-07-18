//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Cancellation-independent Remote Projection pull coordinator. Provider I/O
//! and External Changes scan stay outside the repo permit; apply and guarded
//! finalization each use one serialized repo-lane cut.

use crate::remote_projection_legacy::{self, PreparedProjectionRemotePull};
use crate::server::AppState;
use crate::server::repo_mutation::{
    MountedRepoAdmission, MountedRepoContinuation, RepoMutationPublicationGate,
};
use anyhow::Result;
use deve_core::models::RepoId;
use deve_core::protocol::{RemoteProjectionProvider, ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) struct PullExecutionInput {
    pub(super) state: Arc<AppState>,
    pub(super) gate: Arc<RepoMutationPublicationGate>,
    pub(super) admission: MountedRepoAdmission,
    pub(super) repo_name: String,
    pub(super) repo_id: RepoId,
    pub(super) provider: RemoteProjectionProvider,
    pub(super) locator: String,
}

pub(super) async fn execute<F>(
    input: PullExecutionInput,
    pull_preparer: F,
) -> Result<super::remote_projection::RemoteProjectionExecutionSummary, ServerError>
where
    F: FnOnce(RemoteProjectionProvider, &str) -> Result<PreparedProjectionRemotePull>
        + Send
        + 'static,
{
    let PullExecutionInput {
        state,
        gate,
        admission,
        repo_name,
        repo_id,
        provider,
        locator,
    } = input;
    let prepared = tokio::task::spawn_blocking(move || pull_preparer(provider, &locator))
        .await
        .map_err(provider_error)?
        .map_err(provider_error)?;
    let coordinator = tokio::spawn(async move {
        let repo = state.repo.clone();
        let state_for_apply = state.clone();
        let repo_for_apply = repo.clone();
        let repo_name_for_apply = repo_name.clone();
        let (applied, continuation): (_, MountedRepoContinuation) = gate
            .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(
                admission,
                move || {
                    let bound_name = crate::server::repo_mutation::revalidate_writable_local_repo(
                        &state_for_apply,
                        repo_id,
                        &repo_name_for_apply,
                    )?;
                    remote_projection_legacy::apply_prepared_pull(
                        repo_for_apply,
                        &bound_name,
                        prepared,
                    )
                    .map(|applied| (bound_name, applied.defer_rollback()))
                },
            )
            .await
            .map_err(|error| error.server_error())?;
        let (bound_name, applied) = applied.map_err(provider_error)?;
        let repo_for_scan = repo.clone();
        let scan_name = bound_name.clone();
        let scan_result = match tokio::task::spawn_blocking(move || {
            remote_projection_legacy::scan_prepared_pull(repo_for_scan, &scan_name)
        })
        .await
        {
            Ok(result) => result,
            Err(error) => Err(anyhow::anyhow!(
                "remote projection scan task failed: {error}"
            )),
        };
        gate.execute_mounted_repo_continuation_unpublished_blocking(continuation, move || {
            remote_projection_legacy::finalize_prepared_pull_after_scan(applied, scan_result)
        })
        .await
        .map_err(provider_error)?
        .map_err(provider_error)
    });
    coordinator
        .await
        .map_err(provider_error)?
        .map(super::remote_projection::RemoteProjectionExecutionSummary::from_legacy_pull)
}

fn provider_error(error: impl std::fmt::Display) -> ServerError {
    ServerError::with_detail(
        ServerErrorCode::ScRepoContextInvalid,
        super::remote_projection::remote_projection_provider_io_not_ready_detail(error),
    )
}
