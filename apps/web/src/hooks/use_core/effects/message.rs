//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::{IncomingBatch, WsService, is_current_connection_message};
use crate::i18n::{Locale, t};
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

/// 设置消息处理 Effect。
pub fn setup(ws: &WsService, signals: &CoreSignals, external_changes_refresh: Callback<()>) {
    let ws_rx = ws.clone();
    let signals = *signals;
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));
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
        let schedule_refresh = {
            let changes_refresh = changes_refresh.clone();
            let ws = ws_rx.clone();
            move || {
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
                let timer = Timeout::new(120, move || {
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
            );
            if let Some((response, request_id)) = refresh_response {
                message_projection_recovery::response_completed(
                    response,
                    &request_id,
                    ws_rx,
                    signals,
                    external_changes_refresh,
                    projection_refresh,
                );
            }
        }
        set_last_msg_seq.set(seq);
    }
}

#[cfg(test)]
mod tests {
    use super::{MessageProcessingContext, process_available_messages};
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::state::init_signals;
    use crate::runtime::domain::LoadPhase;
    use deve_core::protocol::{ClientMessage, ServerMessage};
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn metrics_message(uptime_secs: u64, active_connections: u32) -> ServerMessage {
        ServerMessage::SystemMetrics {
            cpu_usage_percent: 12.5,
            memory_used_mb: 128,
            active_connections,
            ops_processed: uptime_secs.saturating_mul(2),
            uptime_secs,
            db_size_bytes: 4096,
            doc_count: 7,
        }
    }

    #[test]
    fn dashboard_metrics_stale_connection_epoch_is_skipped_by_message_effect() {
        let runtime = Owner::new();
        runtime.set();
        let ws = WsService::new_with_incoming_for_test(
            ConnectionStatus::Connected,
            2,
            VecDeque::from([
                (1, 1, metrics_message(10, 1)),
                (2, 2, metrics_message(20, 2)),
            ]),
        );
        let signals = init_signals(ws.status);
        let (last_msg_seq, set_last_msg_seq) = signal(0u64);

        process_available_messages(
            &ws,
            MessageProcessingContext {
                signals,
                locale: crate::i18n::Locale::En,
                last_msg_seq,
                set_last_msg_seq,
                schedule_refresh: &|| {},
                external_changes_refresh: Callback::new(|()| {}),
                projection_refresh: &Default::default(),
            },
        );

        let metrics = signals
            .system_metrics
            .get_untracked()
            .expect("current epoch metrics should be processed");
        assert_eq!(metrics.sample_seq, 1);
        assert_eq!(metrics.uptime_secs, 20);
        assert_eq!(metrics.active_connections, 2);
        assert!(signals.system_metrics_live.get_untracked());
    }

    #[test]
    fn workspace_ingestion_error_from_stale_connection_epoch_is_ignored() {
        let runtime = Owner::new();
        runtime.set();
        let repo_id = deve_core::models::RepoId::new_v4();
        let ws = WsService::new_with_incoming_for_test(
            ConnectionStatus::Connected,
            2,
            VecDeque::from([(
                1,
                1,
                ServerMessage::ProtocolError {
                    error: deve_core::protocol::ServerError::with_detail(
                        deve_core::protocol::ServerErrorCode::StorageWorkspaceIngestionUnavailable,
                        "CANARY_PRIVATE_BACKEND_DETAIL",
                    ),
                    switch_nonce: None,
                    scope_nonce: Some(7),
                },
            )]),
        );
        let signals = init_signals(ws.status);
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(7);
        let (last_msg_seq, set_last_msg_seq) = signal(0u64);

        process_available_messages(
            &ws,
            MessageProcessingContext {
                signals,
                locale: crate::i18n::Locale::En,
                last_msg_seq,
                set_last_msg_seq,
                schedule_refresh: &|| {},
                external_changes_refresh: Callback::new(|()| {}),
                projection_refresh: &Default::default(),
            },
        );

        assert!(!ws.workspace_ingestion_blocked_for_untracked(Some(&repo_id.to_string()), Some(7)));
    }

    #[test]
    fn incoming_gap_retires_epoch_without_processing_retained_suffix() {
        let runtime = Owner::new();
        runtime.set();
        let ws = WsService::new_with_incoming_for_test(
            ConnectionStatus::Connected,
            4,
            VecDeque::from([
                (45, 4, metrics_message(10, 1)),
                (46, 4, metrics_message(20, 2)),
            ]),
        );
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        let signals = init_signals(ws.status);
        let (last_msg_seq, set_last_msg_seq) = signal(0u64);

        process_available_messages(
            &ws,
            MessageProcessingContext {
                signals,
                locale: crate::i18n::Locale::En,
                last_msg_seq,
                set_last_msg_seq,
                schedule_refresh: &|| {},
                external_changes_refresh: Callback::new(|()| {}),
                projection_refresh: &Default::default(),
            },
        );

        assert!(signals.system_metrics.get_untracked().is_none());
        assert_eq!(last_msg_seq.get_untracked(), 46);
        assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert_eq!(ws.drain_connection_controls_for_test().len(), 1);
    }

    #[test]
    fn projection_recovery_coalesces_duplicate_refresh_and_locks_affected_doc() {
        let runtime = Owner::new();
        runtime.set();
        let repo_id = deve_core::models::RepoId::new_v4();
        let doc_id = deve_core::models::DocId::new();
        let required = deve_core::protocol::ProjectionRecoveryRequired {
            repo_id,
            branch: None,
            scope_nonce: Some(7),
            cause: deve_core::protocol::ProjectionRecoveryCause::ExternalApply,
            plan: deve_core::protocol::ProjectionRecoveryPlan::external_apply(vec![doc_id]),
        };
        let ws = WsService::new_with_incoming_for_test(
            ConnectionStatus::Connected,
            4,
            VecDeque::from([
                (
                    1,
                    4,
                    ServerMessage::ProjectionRecoveryRequired(required.clone()),
                ),
                (2, 4, ServerMessage::ProjectionRecoveryRequired(required)),
            ]),
        );
        let signals = init_signals(ws.status);
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(7);
        signals.set_current_doc.set(Some(doc_id));
        let (last_msg_seq, set_last_msg_seq) = signal(0u64);
        let external_refreshes = Arc::new(AtomicUsize::new(0));
        let refresh_counter = external_refreshes.clone();

        process_available_messages(
            &ws,
            MessageProcessingContext {
                signals,
                locale: crate::i18n::Locale::En,
                last_msg_seq,
                set_last_msg_seq,
                schedule_refresh: &|| {},
                external_changes_refresh: Callback::new(move |()| {
                    refresh_counter.fetch_add(1, Ordering::Relaxed);
                }),
                projection_refresh: &Default::default(),
            },
        );

        assert_eq!(signals.load_state.get_untracked(), LoadPhase::Resyncing);
        assert_eq!(external_refreshes.load(Ordering::Relaxed), 1);
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 2);
        assert!(matches!(sent.first(), Some(ClientMessage::ListDocs { .. })));
        assert!(matches!(
            sent.get(1),
            Some(ClientMessage::GetChanges { .. })
        ));
    }
}
