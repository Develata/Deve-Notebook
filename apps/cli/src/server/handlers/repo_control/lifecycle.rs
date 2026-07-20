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
        }
        | RepoLifecycleIntent::Remove {
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
                Err(error) => {
                    tracing::warn!(%error, "repo create projection base admission failed");
                    send_simple_error(
                        channel,
                        request_id,
                        ServerErrorCode::RepoLifecycleInvalidRequest,
                    );
                    return;
                }
            };
            match RepoLifecycleJobIntent::create(&initial_alias, projection_base) {
                Ok(intent) => intent,
                Err(error) => {
                    send_job_error(channel, request_id, &error);
                    return;
                }
            }
        }
        RepoLifecycleIntent::Remove {
            repo_id,
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
            RepoLifecycleJobIntent::remove(repo_id)
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

fn replay_terminal_status(
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
        .publish_lifecycle_settlement(request_id, publication, final_list)
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
) -> anyhow::Result<PathBuf> {
    let selected_id = session.active_repo_id.or(session.last_local_repo_id);
    let execution_name = if let Some(repo_id) = selected_id {
        state
            .repo
            .find_local_repo_name_by_id(repo_id)?
            .ok_or_else(|| anyhow::anyhow!("projection base RepoId is absent from local catalog"))?
    } else {
        state.repo.local_repo_name().to_string()
    };
    Ok(state
        .repo
        .projection_locator_for_local_repo(&execution_name)?
        .projection_base_abs)
}

fn status_response(status: RepoLifecycleJobStatus) -> RepoControlResponse {
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

fn job_error_code(error: &RepoLifecycleJobError) -> ServerErrorCode {
    match error {
        RepoLifecycleJobError::Busy => ServerErrorCode::RepoLifecycleBusy,
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

fn send_job_error(channel: &DualChannel, request_id: Uuid, error: &RepoLifecycleJobError) {
    super::send_error(
        channel,
        request_id,
        job_error_code(error),
        "repo lifecycle request failed",
        error,
    );
}
