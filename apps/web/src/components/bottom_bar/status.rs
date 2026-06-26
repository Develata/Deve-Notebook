//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 07_network#web-ws-runtime
//!   - 18_release#runtime-observability
//!
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::status_summary::{SyncStatusInput, SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use crate::runtime::document::pending::{PendingScope, pending_count_for_doc_in_scope};
use crate::runtime::{
    document_client::DocumentClient, rendering_client::RenderingClient, scope_client::ScopeClient,
    session_client::SessionClient,
};
use leptos::prelude::*;

#[component]
pub fn BottomBarStatus(locale: RwSignal<Locale>) -> impl IntoView {
    let session = expect_context::<SessionClient>();
    let scope = expect_context::<ScopeClient>();
    let document = expect_context::<DocumentClient>();
    let editor = expect_context::<EditorContext>();
    let rendering = expect_context::<RenderingClient>();

    let summary = Memo::new(move |_| {
        let current_doc = document.current_doc.get();
        let current_repo_id = scope.current_repo_id.get();
        let current_scope_nonce = scope.current_scope_nonce.get();
        let pending_ack_count = current_doc
            .and_then(|doc_id| {
                PendingScope::from_repo_id_str(current_repo_id.as_deref(), current_scope_nonce).map(
                    |scope| {
                        pending_count_for_doc_in_scope(
                            &document.pending_local_edits.get(),
                            doc_id,
                            scope,
                        )
                    },
                )
            })
            .unwrap_or_default();
        let handshake_ready = session.handshake_ready.get();
        let readiness = session.ws.native_runtime_readiness_for(
            current_repo_id.as_deref(),
            Some(current_scope_nonce),
            handshake_ready,
        );
        derive_sync_status(SyncStatusInput {
            connection_status: session.connection_status.get(),
            load_state: rendering.load_state.get().as_str(),
            remote_branch_active: scope.active_branch.get().is_some(),
            degraded_storage: scope.is_spectator.get() && scope.active_branch.get().is_none(),
            node_role_probe_failed: session.ws.node_role_probe_failed.get(),
            node_role_readable: readiness.node_role_readable,
            handshake_ready: readiness.repo_handshake_complete,
            writer_ready: readiness.writer_ready,
            current_repo_id: current_repo_id.as_deref(),
            current_repo_name: scope.current_repo.get().as_deref(),
            pending_repo_switch: scope.pending_repo_switch.get().as_deref(),
            pending_branch_switch: editor.pending_branch_switch.get().is_some(),
            pending_ack_count,
        })
    });

    let repo_label = move || {
        let summary = summary.get();
        if matches!(summary.kind, SyncStatusKind::HandshakingRepo) {
            summary.repo_name.unwrap_or_default()
        } else {
            String::new()
        }
    };

    let color = move || match summary.get().kind {
        SyncStatusKind::Ready => "bg-green-500",
        SyncStatusKind::PendingAck => "bg-sky-500",
        SyncStatusKind::ReadOnly | SyncStatusKind::PeerNotRegistered => "bg-slate-400",
        SyncStatusKind::HandshakingRepo
        | SyncStatusKind::Reconnecting
        | SyncStatusKind::NativeSessionPending
        | SyncStatusKind::NativeReprobeRequired => "bg-yellow-500",
        SyncStatusKind::SnapshotLoading => "bg-blue-500",
        SyncStatusKind::SessionExpired => "bg-amber-500",
        SyncStatusKind::NativeBootstrapInvalid
        | SyncStatusKind::NativeServiceOffline
        | SyncStatusKind::Offline => "bg-red-500",
    };

    let text = move || {
        let summary = summary.get();
        match summary.kind {
            SyncStatusKind::Ready => t::bottom_bar::ready(locale.get()).to_string(),
            SyncStatusKind::PendingAck => {
                t::bottom_bar::pending_ack(locale.get(), summary.pending_ack_count)
            }
            SyncStatusKind::ReadOnly => t::bottom_bar::read_only(locale.get()).to_string(),
            SyncStatusKind::HandshakingRepo => {
                t::bottom_bar::handshaking_repo(locale.get()).to_string()
            }
            SyncStatusKind::PeerNotRegistered => {
                t::bottom_bar::peer_not_registered(locale.get()).to_string()
            }
            SyncStatusKind::SnapshotLoading => {
                t::bottom_bar::snapshot_loading(locale.get()).to_string()
            }
            SyncStatusKind::Reconnecting => t::bottom_bar::reconnecting(locale.get()).to_string(),
            SyncStatusKind::SessionExpired => t::bottom_bar::unauthorized(locale.get()).to_string(),
            SyncStatusKind::NativeBootstrapInvalid => {
                t::bottom_bar::native_bootstrap_invalid(locale.get()).to_string()
            }
            SyncStatusKind::NativeSessionPending => {
                t::bottom_bar::native_session_pending(locale.get()).to_string()
            }
            SyncStatusKind::NativeServiceOffline => {
                t::bottom_bar::native_service_offline(locale.get()).to_string()
            }
            SyncStatusKind::NativeReprobeRequired => {
                t::bottom_bar::native_reprobe_required(locale.get()).to_string()
            }
            SyncStatusKind::Offline => t::bottom_bar::offline(locale.get()).to_string(),
        }
    };

    view! {
        <div
            class="flex items-center gap-2 min-w-0"
            data-deve-sync-status=move || sync_status_kind_marker(summary.get().kind)
        >
            <div class=move || format!("w-2 h-2 rounded-full {}", color())></div>
            <span class="text-xs text-secondary font-medium">{text}</span>
            <Show when=move || matches!(summary.get().kind, SyncStatusKind::PeerNotRegistered)>
                <button
                    type="button"
                    class="text-[10px] text-accent hover:text-accent-hover underline underline-offset-2"
                    data-deve-peer-registration-retry="true"
                    aria-label=move || t::bottom_bar::retry_peer_registration(locale.get())
                    on:click=move |_| session.on_retry_peer_registration.run(())
                >
                    {move || t::bottom_bar::retry_peer_registration(locale.get())}
                </button>
            </Show>
            <Show when=move || !repo_label().is_empty()>
                <span class="text-[10px] text-muted font-mono truncate">{repo_label}</span>
            </Show>
        </div>
    }
}

fn sync_status_kind_marker(kind: SyncStatusKind) -> &'static str {
    match kind {
        SyncStatusKind::SessionExpired => "session-expired",
        SyncStatusKind::NativeBootstrapInvalid => "native-bootstrap-invalid",
        SyncStatusKind::NativeSessionPending => "native-session-pending",
        SyncStatusKind::NativeServiceOffline => "native-service-offline",
        SyncStatusKind::NativeReprobeRequired => "native-reprobe-required",
        SyncStatusKind::Offline => "offline",
        SyncStatusKind::Reconnecting => "reconnecting",
        SyncStatusKind::SnapshotLoading => "snapshot-loading",
        SyncStatusKind::ReadOnly => "read-only",
        SyncStatusKind::HandshakingRepo => "handshaking-repo",
        SyncStatusKind::PeerNotRegistered => "peer-not-registered",
        SyncStatusKind::PendingAck => "pending-ack",
        SyncStatusKind::Ready => "ready",
    }
}
