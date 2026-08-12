//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::{IncomingBatch, WsService, is_current_connection_message};
use crate::i18n::{Locale, t};
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use crate::runtime::projection_recovery::{ProjectionRefreshCoordinator, ProjectionRefreshScope};
use deve_core::protocol::ClientMessage;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::state::CoreSignals;
use super::super::write_gate::RepoWriteGateState;
use super::message_dispatch;
use super::message_projection_recovery;
use super::message_refresh::{capture_refresh_scope, should_send_refresh_through_read_gate};
use crate::runtime::remote_import_client::RemoteImportClient;
use crate::runtime::repo_control_client::RepoControlClient;

/// 设置消息处理 Effect。
pub fn setup(
    ws: &WsService,
    signals: &CoreSignals,
    external_changes_refresh: Callback<()>,
    repo_control: RepoControlClient,
    remote_import: RemoteImportClient,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    let ws_rx = ws.clone();
    let signals = *signals;
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));
    let changes_refresh_cleanup = StoredValue::new_local(changes_refresh.clone());
    on_cleanup(move || {
        changes_refresh_cleanup.with_value(|changes_refresh| {
            if let Some(timeout) = changes_refresh.borrow_mut().take() {
                timeout.cancel();
            }
        });
    });
    let (last_msg_seq, set_last_msg_seq) = signal(0u64);
    let projection_refresh = ProjectionRefreshCoordinator::default();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    Effect::new(move |_| {
        let locale = locale.get();
        let banner = degraded_sync_mode
            .get()
            .map(|mode| t::bottom_bar::storage_limited_read_only(locale, mode.blocker).to_string());
        set_sync_banner.set(banner);
    });

    Effect::new(move |_| {
        if !runtime_lifetime.is_active() {
            return;
        }
        let schedule_refresh = {
            let changes_refresh = changes_refresh.clone();
            let ws = ws_rx.clone();
            let runtime_lifetime = runtime_lifetime.clone();
            move || {
                if !runtime_lifetime.is_active() {
                    return;
                }
                let refresh_scope = capture_refresh_scope(
                    signals.current_repo_id.get_untracked(),
                    signals.active_branch.get_untracked(),
                    signals
                        .pending_branch_switch
                        .get_untracked()
                        .map(|pending| pending.into_target()),
                    signals
                        .pending_repo_switch
                        .get_untracked()
                        .map(|pending| pending.expected_name),
                    signals.current_scope_nonce.get_untracked(),
                );
                let Some(refresh_scope) = refresh_scope else {
                    return;
                };
                if let Some(t) = changes_refresh.borrow_mut().take() {
                    t.cancel();
                }
                let ws_for_timer = ws.clone();
                let set_changes_request_id = signals.set_changes_request_id;
                let current_repo_id = signals.current_repo_id;
                let active_branch = signals.active_branch;
                let pending_branch_switch = signals.pending_branch_switch;
                let pending_repo_switch = signals.pending_repo_switch;
                let timer_lifetime = runtime_lifetime.clone();
                let timer = Timeout::new(120, move || {
                    if !timer_lifetime.is_active() {
                        return;
                    }
                    let repo_id = current_repo_id.get_untracked();
                    let branch = active_branch.get_untracked();
                    let is_remote_branch_read = branch.is_some();
                    let pending_branch = pending_branch_switch.get_untracked();
                    let pending_repo = pending_repo_switch.get_untracked();
                    let pending_branch_target =
                        pending_branch.clone().map(|pending| pending.into_target());
                    let pending_repo_name =
                        pending_repo.clone().map(|pending| pending.expected_name);
                    let scope_nonce = signals.current_scope_nonce.get_untracked();
                    let load_state = signals.load_state.get_untracked();
                    let readiness = ws_for_timer.native_runtime_readiness_for_untracked(
                        repo_id.as_deref(),
                        Some(scope_nonce),
                        signals.handshake_ready.get_untracked(),
                    );
                    if !should_send_refresh_through_read_gate(
                        &refresh_scope,
                        repo_id.clone(),
                        branch,
                        pending_branch_target,
                        pending_repo_name,
                        scope_nonce,
                        RepoWriteGateState {
                            connection_status: ws_for_timer.status.get_untracked(),
                            load_state: load_state.as_str(),
                            is_read_only: signals.is_spectator.get_untracked()
                                || is_remote_branch_read,
                            node_role_probe_failed: ws_for_timer
                                .node_role_probe_failed
                                .get_untracked(),
                            node_role_readable: readiness.node_role_readable,
                            handshake_ready: readiness.repo_handshake_complete,
                            writer_ready: readiness.writer_ready,
                            has_repo: repo_id.is_some(),
                            workspace_ingestion_blocked: ws_for_timer
                                .workspace_ingestion_blocked_for_untracked(
                                    repo_id.as_deref(),
                                    Some(scope_nonce),
                                ),
                            pending_branch_switch: pending_branch.is_some(),
                            pending_repo_switch: pending_repo.is_some(),
                        },
                    ) {
                        return;
                    }
                    let request_id = uuid::Uuid::new_v4().to_string();
                    set_changes_request_id.set(Some(request_id.clone()));
                    ws_for_timer.send(ClientMessage::GetChanges {
                        request_id,
                        scope_nonce: Some(scope_nonce),
                    });
                });
                *changes_refresh.borrow_mut() = Some(timer);
            }
        };

        let _ = ws_rx.msg_seq.get();
        process_available_messages(
            &ws_rx,
            MessageProcessingContext {
                signals,
                locale: locale.get_untracked(),
                last_msg_seq,
                set_last_msg_seq,
                schedule_refresh: &schedule_refresh,
                external_changes_refresh,
                projection_refresh: &projection_refresh,
                repo_control: &repo_control,
                remote_import: &remote_import,
                runtime_lifetime: &runtime_lifetime,
            },
        );
    });
}

struct MessageProcessingContext<'a> {
    signals: CoreSignals,
    locale: Locale,
    last_msg_seq: ReadSignal<u64>,
    set_last_msg_seq: WriteSignal<u64>,
    schedule_refresh: &'a dyn Fn(),
    external_changes_refresh: Callback<()>,
    projection_refresh: &'a ProjectionRefreshCoordinator,
    repo_control: &'a RepoControlClient,
    remote_import: &'a RemoteImportClient,
    runtime_lifetime: &'a BrowserRuntimeLifetime,
}

fn process_available_messages(ws_rx: &WsService, context: MessageProcessingContext<'_>) {
    let MessageProcessingContext {
        signals,
        locale,
        last_msg_seq,
        set_last_msg_seq,
        schedule_refresh,
        external_changes_refresh,
        projection_refresh,
        repo_control,
        remote_import,
        runtime_lifetime,
    } = context;
    let current_connection_epoch = ws_rx.connection_epoch.get_untracked();
    projection_refresh.enter_scope(ProjectionRefreshScope {
        connection_epoch: current_connection_epoch,
        repo_id: signals
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse().ok()),
        branch: signals.active_branch.get_untracked(),
        scope_nonce: signals.current_scope_nonce.get_untracked(),
        scope_switch_pending: signals.pending_repo_switch.get_untracked().is_some()
            || signals.pending_branch_switch.get_untracked().is_some(),
    });
    if ws_rx.reconnect_for_resync_pending(current_connection_epoch) {
        set_last_msg_seq.set(ws_rx.msg_seq.get_untracked());
        return;
    }
    let messages = match ws_rx.messages_since(last_msg_seq.get_untracked()) {
        IncomingBatch::Messages(messages) => messages,
        IncomingBatch::Gap { latest_seq } => {
            set_last_msg_seq.set(latest_seq);
            signals
                .set_load_state
                .set(crate::runtime::domain::LoadPhase::Resyncing);
            ws_rx.request_reconnect_for_resync(current_connection_epoch);
            return;
        }
    };
    for (seq, connection_epoch, msg) in messages {
        if !is_current_connection_message(connection_epoch, current_connection_epoch) {
            set_last_msg_seq.set(seq);
            continue;
        }
        if let deve_core::protocol::ServerMessage::ProjectionRecoveryRequired(required) = msg {
            message_projection_recovery::handle_required(
                required,
                ws_rx,
                signals,
                external_changes_refresh,
                projection_refresh,
                runtime_lifetime.clone(),
            );
        } else {
            let refresh_response = message_projection_recovery::capture_response(&msg);
            message_projection_recovery::retire_failed_refresh(
                &msg,
                ws_rx,
                signals,
                projection_refresh,
            );
            message_dispatch::handle_message(
                msg,
                ws_rx,
                signals,
                locale,
                schedule_refresh,
                external_changes_refresh,
                message_dispatch::MessageDispatchClients {
                    repo_control,
                    remote_import,
                },
            );
            if let Some((response, request_id)) = refresh_response {
                message_projection_recovery::response_completed(
                    response,
                    &request_id,
                    ws_rx,
                    signals,
                    external_changes_refresh,
                    projection_refresh,
                    runtime_lifetime.clone(),
                );
            }
        }
        set_last_msg_seq.set(seq);
    }
}

#[cfg(test)]
mod tests;
