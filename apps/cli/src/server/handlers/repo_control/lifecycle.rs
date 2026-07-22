//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract
//!
//! SubmitLifecycle / GetLifecycle transport arms: observer registration,
//! idempotent submission to the host-owned job runtime, and exact terminal
//! settlement replay for reconnecting observers.

use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobError, RepoLifecycleJobIntent, RepoLifecycleJobOperation,
    RepoLifecycleJobOutcome, RepoLifecycleJobPhase, RepoLifecycleJobStatus,
};
use crate::server::runtime::repo_session_runtime::FinalRepoListProjection;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::RepoId;
use deve_core::protocol::{
    RepoControlResponse, RepoLifecycleIntent, RepoLifecycleOperation, RepoLifecycleOutcome,
    RepoLifecycleState, ServerErrorCode, ServerMessage,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use super::send_simple_error;

pub(super) async fn handle_submit_lifecycle(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
    lifecycle_intent: RepoLifecycleIntent,
) {
    let (expected_scope_nonce, observer_switch_nonce) = match &lifecycle_intent {
        RepoLifecycleIntent::Create {
            current_scope_nonce,
            switch_nonce,
            ..
        } => (current_scope_nonce.get(), switch_nonce.get()),
    };
    let intent = match lifecycle_intent {
        RepoLifecycleIntent::Create {
            initial_alias,
            current_scope_nonce,
            switch_nonce,
        } => {
            if !valid_lifecycle_observer(session, current_scope_nonce.get(), switch_nonce.get())
                || session.active_branch.is_some()
            {
                send_simple_error(
                    channel,
                    request_id,
                    ServerErrorCode::RepoLifecycleInvalidRequest,
                );
                return;
            }
            let projection_base = match projection_base_for_new_repo(state, session) {
                Ok(base) => base,
                Err(ProjectionBaseAdmissionError::Required) => {
                    send_simple_error(
                        channel,
                        request_id,
                        ServerErrorCode::RepoCreationProjectionBaseRequired,
                    );
                    return;
                }
                Err(ProjectionBaseAdmissionError::Invalid(error)) => {
                    tracing::warn!(%error, "repo create projection base admission failed");
                    send_simple_error(
                        channel,
                        request_id,
                        ServerErrorCode::RepoLifecycleInvalidRequest,
                    );
                    return;
                }
            };
            match RepoLifecycleJobIntent::create(
                &initial_alias,
                projection_base.path,
                projection_base.source_repo_id,
            ) {
                Ok(intent) => intent,
                Err(error) => {
                    send_job_error(channel, request_id, &error);
                    return;
                }
            }
        }
    };
    let Some(session_id) = session.repo_session_runtime_id() else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    };
    if let Err(error) = state.repo_session_runtime().register_lifecycle_observer(
        session_id,
        request_id,
        expected_scope_nonce,
        observer_switch_nonce,
    ) {
        tracing::warn!(%error, "repo lifecycle observer registration failed");
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleRepairRequired,
        );
        return;
    }
    match state.repo_lifecycle_jobs().submit(request_id, intent).await {
        Ok(accepted) => {
            channel.unicast(ServerMessage::RepoControl(
                RepoControlResponse::LifecycleAccepted {
                    request_id: accepted.request_id,
                    job_id: accepted.job_id,
                    target_repo_id: accepted.target_repo_id,
                },
            ));
            // An idempotent retry may address a terminal receipt whose
            // original publication happened before this observer was
            // registered. Re-read typed truth and replay that exact
            // settlement instead of leaving the new connection pending.
            match state.repo_lifecycle_jobs().status(request_id).await {
                Ok(status) if status.phase == RepoLifecycleJobPhase::Terminal => {
                    if let Err(code) =
                        replay_terminal_status(state, session_id, request_id, &status)
                    {
                        send_simple_error(channel, request_id, code);
                        return;
                    }
                    channel.unicast(ServerMessage::RepoControl(status_response(status)));
                }
                Ok(_) => {}
                Err(error) => {
                    let _ = state
                        .repo_session_runtime()
                        .clear_lifecycle_observer(session_id, request_id);
                    send_job_error(channel, request_id, &error);
                }
            }
        }
        Err(error) => {
            let _ = state
                .repo_session_runtime()
                .clear_lifecycle_observer(session_id, request_id);
            send_job_error(channel, request_id, &error);
        }
    }
}

pub(super) async fn handle_get_lifecycle(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
) {
    let Some(session_id) = session.repo_session_runtime_id() else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    };
    let Some(switch_nonce) = session.scope_nonce().checked_add(1) else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    };
    if let Err(error) = state.repo_session_runtime().register_lifecycle_observer(
        session_id,
        request_id,
        session.scope_nonce(),
        switch_nonce,
    ) {
        tracing::warn!(%error, "lifecycle reconnect observer registration failed");
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleRepairRequired,
        );
        return;
    }
    let status = match state.repo_lifecycle_jobs().status(request_id).await {
        Ok(status) => status,
        Err(error) => {
            let _ = state
                .repo_session_runtime()
                .clear_lifecycle_observer(session_id, request_id);
            send_job_error(channel, request_id, &error);
            return;
        }
    };
    if let Err(code) = replay_terminal_status(state, session_id, request_id, &status) {
        send_simple_error(channel, request_id, code);
        return;
    }
    channel.unicast(ServerMessage::RepoControl(status_response(status)));
}

pub(super) fn replay_terminal_status(
    state: &Arc<AppState>,
    session_id: u64,
    request_id: Uuid,
    status: &RepoLifecycleJobStatus,
) -> Result<(), ServerErrorCode> {
    if status.phase != RepoLifecycleJobPhase::Terminal {
        return Ok(());
    }
    let Some(publication) = status.publication.clone() else {
        let _ = state
            .repo_session_runtime()
            .clear_lifecycle_observer(session_id, request_id);
        return Ok(());
    };
    let final_list = crate::server::handlers::repo_list::local_repo_list_entries(state)
        .map(|entries| FinalRepoListProjection { entries })
        .map_err(|error| {
            tracing::warn!(%error, "lifecycle observer list projection failed");
            let _ = state
                .repo_session_runtime()
                .clear_lifecycle_observer(session_id, request_id);
            ServerErrorCode::RepoLifecycleRepairRequired
        })?;
    state
        .repo_session_runtime()
        .publish_lifecycle_settlement(request_id, status.job_id, publication, final_list)
        .map_err(|error| {
            tracing::warn!(%error, "lifecycle observer publication replay failed");
            let _ = state
                .repo_session_runtime()
                .clear_lifecycle_observer(session_id, request_id);
            ServerErrorCode::RepoLifecycleRepairRequired
        })?;
    Ok(())
}

fn valid_lifecycle_observer(
    session: &WsSession,
    current_scope_nonce: u64,
    switch_nonce: u64,
) -> bool {
    session.is_browser_session()
        && session.scope_nonce() == current_scope_nonce
        && switch_nonce > current_scope_nonce
}

fn projection_base_for_new_repo(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<PreparedProjectionBaseBinding, ProjectionBaseAdmissionError> {
    projection_base_for_new_repo_with_config(state, session, state.repo_creation_projection_base())
}

fn projection_base_for_new_repo_with_config(
    state: &Arc<AppState>,
    session: &WsSession,
    configured_base: Option<&std::path::Path>,
) -> Result<PreparedProjectionBaseBinding, ProjectionBaseAdmissionError> {
    let selected_id = session.active_repo_id.or(session.last_local_repo_id);
    if let Some(repo_id) = selected_id {
        let execution_name = state
            .repo
            .find_local_repo_name_by_id(repo_id)?
            .ok_or_else(|| {
                ProjectionBaseAdmissionError::Invalid(anyhow::anyhow!(
                    "projection base RepoId is absent from local catalog"
                ))
            })?;
        return Ok(PreparedProjectionBaseBinding {
            path: state
                .repo
                .projection_locator_for_local_repo(&execution_name)?
                .projection_base_abs,
            source_repo_id: Some(repo_id),
        });
    }
    configured_base
        .map(|path| PreparedProjectionBaseBinding {
            path: PathBuf::from(path),
            source_repo_id: None,
        })
        .ok_or(ProjectionBaseAdmissionError::Required)
}

#[derive(Debug, PartialEq, Eq)]
struct PreparedProjectionBaseBinding {
    path: PathBuf,
    source_repo_id: Option<RepoId>,
}

#[derive(Debug)]
enum ProjectionBaseAdmissionError {
    Required,
    Invalid(anyhow::Error),
}

impl std::fmt::Display for ProjectionBaseAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required => formatter.write_str("repo creation projection base is required"),
            Self::Invalid(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProjectionBaseAdmissionError {}

impl From<anyhow::Error> for ProjectionBaseAdmissionError {
    fn from(error: anyhow::Error) -> Self {
        Self::Invalid(error)
    }
}

pub(super) fn status_response(status: RepoLifecycleJobStatus) -> RepoControlResponse {
    RepoControlResponse::LifecycleStatus {
        request_id: status.request_id,
        job_id: status.job_id,
        target_repo_id: status.target_repo_id,
        operation: match status.operation {
            RepoLifecycleJobOperation::Create => RepoLifecycleOperation::Create,
            RepoLifecycleJobOperation::Remove => RepoLifecycleOperation::Remove,
        },
        state: match status.phase {
            RepoLifecycleJobPhase::Accepted => RepoLifecycleState::Accepted,
            RepoLifecycleJobPhase::Running => RepoLifecycleState::Running,
            RepoLifecycleJobPhase::Recovering => RepoLifecycleState::Recovering,
            RepoLifecycleJobPhase::Terminal => RepoLifecycleState::Terminal,
        },
        outcome: status.outcome.map(|outcome| match outcome {
            RepoLifecycleJobOutcome::Succeeded => RepoLifecycleOutcome::Succeeded,
            RepoLifecycleJobOutcome::NotCommitted => RepoLifecycleOutcome::NotCommitted,
            RepoLifecycleJobOutcome::CommittedPartial => RepoLifecycleOutcome::CommittedPartial,
            RepoLifecycleJobOutcome::RepairRequired => RepoLifecycleOutcome::RepairRequired,
        }),
        publication_pending: status.publication_pending,
    }
}

pub(super) fn job_error_code(error: &RepoLifecycleJobError) -> ServerErrorCode {
    match error {
        RepoLifecycleJobError::Busy => ServerErrorCode::RepoLifecycleBusy,
        RepoLifecycleJobError::RemovalBlocked => ServerErrorCode::RepoLifecycleRemovalBlocked,
        RepoLifecycleJobError::ConfirmationInvalid => {
            ServerErrorCode::RepoLifecycleConfirmationInvalid
        }
        RepoLifecycleJobError::ConfirmationExpired => {
            ServerErrorCode::RepoLifecycleConfirmationExpired
        }
        RepoLifecycleJobError::ConfirmationStale => ServerErrorCode::RepoLifecycleConfirmationStale,
        RepoLifecycleJobError::NotFound => ServerErrorCode::RepoLifecycleNotFound,
        RepoLifecycleJobError::InvalidRequest | RepoLifecycleJobError::RequestConflict => {
            ServerErrorCode::RepoLifecycleInvalidRequest
        }
        RepoLifecycleJobError::AdmissionClosed
        | RepoLifecycleJobError::Store(_)
        | RepoLifecycleJobError::Coordination(_)
        | RepoLifecycleJobError::Shutdown(_) => ServerErrorCode::RepoLifecycleRepairRequired,
    }
}

pub(super) fn send_job_error(
    channel: &DualChannel,
    request_id: Uuid,
    error: &RepoLifecycleJobError,
) {
    super::send_error(
        channel,
        request_id,
        job_error_code(error),
        "repo lifecycle request failed",
        error,
    );
}

#[cfg(test)]
mod tests {
    use super::{ProjectionBaseAdmissionError, projection_base_for_new_repo_with_config};
    use crate::server::runtime::repo_lifecycle_runtime::{CreateRepoIntent, RepoMountOutcome};
    use crate::server::switcher_test_support::{app_state, browser_session};
    use deve_core::ledger::RepoManager;
    use deve_core::models::RepoId;

    #[tokio::test]
    async fn zero_repo_host_starts_no_scope_and_creates_from_configured_base() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        std::fs::create_dir_all(&projection_base)?;
        let repo = RepoManager::init_empty_host(&ledger, 8)?;
        let state = app_state(repo, projection_base.clone(), dir.path().join("host"))?;
        assert!(state.repo.list_cataloged_local_repo_summaries()?.is_empty());

        let session = browser_session(1);
        let resolved =
            projection_base_for_new_repo_with_config(&state, &session, Some(&projection_base))?;
        assert_eq!(resolved.source_repo_id, None);
        let repo_id = RepoId::new_v4();
        let outcome = state
            .repo_lifecycle_coordinator()
            .create(CreateRepoIntent {
                repo_id,
                initial_alias: "first repo".to_string(),
                projection_base: resolved.path,
                lifecycle_request_id: uuid::Uuid::new_v4(),
            })
            .await?;

        assert_eq!(outcome.mount, RepoMountOutcome::Mounted);
        assert_eq!(state.repo.list_cataloged_local_repo_summaries()?.len(), 1);
        assert_eq!(state.repo.current_local_repo_name()?, repo_id.to_string());
        assert!(
            state
                .repo
                .check_projection_locator_for_local_repo(&repo_id.to_string())?
                .starts_with(std::fs::canonicalize(&projection_base)?)
        );
        state
            .repo_lifecycle_coordinator()
            .shutdown_watchers_for_test();
        Ok(())
    }

    #[test]
    fn zero_repo_create_without_projection_base_is_typed_before_ws_v5_projection()
    -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let repo = RepoManager::init_empty_host(dir.path().join("ledger"), 8)?;
        let state = app_state(repo, dir.path().join("unused"), dir.path().join("host"))?;
        let session = browser_session(1);

        assert!(matches!(
            projection_base_for_new_repo_with_config(&state, &session, None),
            Err(ProjectionBaseAdmissionError::Required)
        ));
        Ok(())
    }
}
