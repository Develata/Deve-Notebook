//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 05_network#web-ws-runtime
//!   - 15_release#runtime-observability
//!
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::status_summary::{SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn BottomBarStatus(core: CoreState, locale: RwSignal<Locale>) -> impl IntoView {
    let summary = Memo::new(move |_| {
        let current_doc = core.current_doc.get();
        let pending_ack_count = current_doc
            .and_then(|doc_id| core.pending_local_edits.get().get(&doc_id).map(Vec::len))
            .unwrap_or_default();
        let current_repo_id = core.current_repo_id.get();
        let current_scope_nonce = core.current_scope_nonce.get();
        let handshake_ready = core.handshake_ready.get();
        let readiness = core.ws.native_runtime_readiness_for(
            current_repo_id.as_deref(),
            Some(current_scope_nonce),
            handshake_ready,
        );
        derive_sync_status(
            core.ws.status.get(),
            &core.load_state.get(),
            core.active_branch.get().is_some(),
            core.is_spectator.get() && core.active_branch.get().is_none(),
            core.ws.node_role_probe_failed.get(),
            readiness.node_role_readable,
            readiness.repo_handshake_complete,
            readiness.writer_ready,
            current_repo_id.as_deref(),
            core.current_repo.get().as_deref(),
            core.pending_repo_switch.get().as_deref(),
            core.pending_branch_switch.get().is_some(),
            pending_ack_count,
        )
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
        SyncStatusKind::ReadOnly => "bg-slate-400",
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
        <div class="flex items-center gap-2 min-w-0">
            <div class=move || format!("w-2 h-2 rounded-full {}", color())></div>
            <span class="text-xs text-secondary font-medium">{text}</span>
            <Show when=move || !repo_label().is_empty()>
                <span class="text-[10px] text-muted font-mono truncate">{repo_label}</span>
            </Show>
        </div>
    }
}
