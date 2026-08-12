//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! 消息 effect 的连接代际、gap 与投影恢复回归合同。

use super::{MessageProcessingContext, process_available_messages};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::state::init_signals;
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use crate::runtime::domain::LoadPhase;
use deve_core::protocol::{ClientMessage, ServerMessage};
use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn fixture_remote_import(
    ws: &WsService,
    signals: crate::hooks::use_core::state::CoreSignals,
) -> crate::runtime::remote_import_client::RemoteImportClient {
    crate::runtime::remote_import_client::RemoteImportClient::new(
        ws.clone(),
        signals.current_repo_id,
        signals.active_branch,
        signals.current_scope_nonce,
        signals.pending_branch_switch,
        signals.pending_repo_switch,
    )
}

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
            repo_control: &Default::default(),
            remote_import: &fixture_remote_import(&ws, signals),
            runtime_lifetime: &BrowserRuntimeLifetime::new(),
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
            repo_control: &Default::default(),
            remote_import: &fixture_remote_import(&ws, signals),
            runtime_lifetime: &BrowserRuntimeLifetime::new(),
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
            repo_control: &Default::default(),
            remote_import: &fixture_remote_import(&ws, signals),
            runtime_lifetime: &BrowserRuntimeLifetime::new(),
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
            repo_control: &Default::default(),
            remote_import: &fixture_remote_import(&ws, signals),
            runtime_lifetime: &BrowserRuntimeLifetime::new(),
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
