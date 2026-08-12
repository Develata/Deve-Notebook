//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
//! Hooks adapter that executes backend-specified projection refresh intents.

use crate::api::WsService;
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use crate::runtime::domain::LoadPhase;
use crate::runtime::projection_recovery::{
    ProjectionRecoveryScope, ProjectionRefreshCoordinator, ProjectionRefreshResponse,
    ProjectionRefreshWork, evaluate_recovery,
};
use deve_core::protocol::{ClientMessage, ProjectionRecoveryRequired, ServerMessage};
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::{Callable, Callback, GetUntracked, Set};
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;

#[cfg(target_arch = "wasm32")]
const PROJECTION_REFRESH_TIMEOUT_MS: u32 = 10_000;

use super::super::state::CoreSignals;

pub(super) fn handle_required(
    required: ProjectionRecoveryRequired,
    ws: &WsService,
    signals: CoreSignals,
    external_changes_refresh: Callback<()>,
    coordinator: &ProjectionRefreshCoordinator,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    let Some(decision) = evaluate_recovery(&required, &recovery_scope(signals)) else {
        return;
    };

    if decision.current_document_affected {
        signals.set_load_state.set(LoadPhase::Resyncing);
    }
    if !decision.requires_refresh() {
        return;
    }
    if let Some(work) = coordinator.begin(required, decision.plan) {
        execute_refresh_work(
            work,
            ws,
            signals,
            external_changes_refresh,
            coordinator,
            runtime_lifetime,
        );
    }
}

pub(super) fn capture_response(msg: &ServerMessage) -> Option<(ProjectionRefreshResponse, String)> {
    match msg {
        ServerMessage::DocList {
            request_id: Some(request_id),
            ..
        } => Some((ProjectionRefreshResponse::DocList, request_id.clone())),
        ServerMessage::ChangesList {
            request_id: Some(request_id),
            ..
        } => Some((ProjectionRefreshResponse::SourceControl, request_id.clone())),
        _ => None,
    }
}

pub(super) fn response_completed(
    response: ProjectionRefreshResponse,
    request_id: &str,
    ws: &WsService,
    signals: CoreSignals,
    external_changes_refresh: Callback<()>,
    coordinator: &ProjectionRefreshCoordinator,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    if let Some(work) = coordinator.complete_response(response, request_id) {
        execute_refresh_work(
            work,
            ws,
            signals,
            external_changes_refresh,
            coordinator,
            runtime_lifetime,
        );
    }
}

pub(super) fn retire_failed_refresh(
    msg: &ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    coordinator: &ProjectionRefreshCoordinator,
) {
    let ServerMessage::ProtocolError { scope_nonce, .. } = msg else {
        return;
    };
    if *scope_nonce != Some(signals.current_scope_nonce.get_untracked())
        || !coordinator.retire_active()
    {
        return;
    }
    signals.set_load_state.set(LoadPhase::Resyncing);
    ws.request_reconnect_for_resync(ws.connection_epoch.get_untracked());
}

fn execute_refresh_work(
    work: ProjectionRefreshWork,
    ws: &WsService,
    signals: CoreSignals,
    external_changes_refresh: Callback<()>,
    coordinator: &ProjectionRefreshCoordinator,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    if !runtime_lifetime.is_active() {
        return;
    }
    if evaluate_recovery(&work.required, &recovery_scope(signals)).is_none() {
        return;
    }
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let doc_list_request_id = if work.plan.refresh_doc_list {
        let request_id = uuid::Uuid::new_v4().to_string();
        signals
            .set_doc_list_request_id
            .set(Some(request_id.clone()));
        signals.set_tree_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::ListDocs {
            request_id: request_id.clone(),
            scope_nonce: Some(scope_nonce),
        });
        Some(request_id)
    } else {
        None
    };
    let source_control_request_id = if work.plan.refresh_source_control {
        let request_id = uuid::Uuid::new_v4().to_string();
        signals.set_changes_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetChanges {
            request_id: request_id.clone(),
            scope_nonce: Some(scope_nonce),
        });
        Some(request_id)
    } else {
        None
    };
    if work.plan.refresh_external_changes {
        external_changes_refresh.run(());
    }
    #[cfg(target_arch = "wasm32")]
    schedule_refresh_timeout(
        work.flight_id,
        ws,
        signals,
        coordinator,
        runtime_lifetime.clone(),
    );
    if let Some(trailing) = coordinator.register_requests(
        work.flight_id,
        doc_list_request_id,
        source_control_request_id,
    ) {
        execute_refresh_work(
            trailing,
            ws,
            signals,
            external_changes_refresh,
            coordinator,
            runtime_lifetime,
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn schedule_refresh_timeout(
    flight_id: u64,
    ws: &WsService,
    signals: CoreSignals,
    coordinator: &ProjectionRefreshCoordinator,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    let ws = ws.clone();
    let coordinator = coordinator.clone();
    let connection_epoch = ws.connection_epoch.get_untracked();
    spawn_local(async move {
        TimeoutFuture::new(PROJECTION_REFRESH_TIMEOUT_MS).await;
        if !runtime_lifetime.is_active() {
            return;
        }
        if coordinator.retire(flight_id) {
            signals.set_load_state.set(LoadPhase::Resyncing);
            ws.request_reconnect_for_resync(connection_epoch);
        }
    });
}

fn recovery_scope(signals: CoreSignals) -> ProjectionRecoveryScope {
    ProjectionRecoveryScope {
        repo_id: signals
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse().ok()),
        branch: signals.active_branch.get_untracked(),
        scope_nonce: signals.current_scope_nonce.get_untracked(),
        current_doc: signals.current_doc.get_untracked(),
        scope_switch_pending: signals.pending_repo_switch.get_untracked().is_some()
            || signals.pending_branch_switch.get_untracked().is_some(),
    }
}
