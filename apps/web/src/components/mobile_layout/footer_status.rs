// apps/web/src/components/mobile_layout/footer_status.rs
//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 15_release#runtime-observability
//!
//! # Mobile Footer — Status & Load Indicators

use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::status_summary::{SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// Connection status indicator (green/yellow/red dot + text).
#[component]
pub fn StatusView(core: CoreState, locale: RwSignal<Locale>) -> impl IntoView {
    move || {
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
        let summary = derive_sync_status(
            core.ws.status.get(),
            &core.load_state.get(),
            core.active_branch.get().is_some(),
            core.is_spectator.get() && core.active_branch.get().is_none(),
            readiness.node_role_readable,
            readiness.repo_handshake_complete,
            readiness.writer_ready,
            current_repo_id.as_deref(),
            core.current_repo.get().as_deref(),
            core.pending_repo_switch.get().as_deref(),
            core.pending_branch_switch.get().is_some(),
            pending_ack_count,
        );
        let (color, text) = match summary.kind {
            SyncStatusKind::Ready => (
                "bg-green-500",
                t::bottom_bar::ready(locale.get()).to_string(),
            ),
            SyncStatusKind::PendingAck => (
                "bg-sky-500",
                t::bottom_bar::pending_ack(locale.get(), summary.pending_ack_count),
            ),
            SyncStatusKind::ReadOnly => (
                "bg-slate-400",
                t::bottom_bar::read_only(locale.get()).to_string(),
            ),
            SyncStatusKind::HandshakingRepo => (
                "bg-yellow-500",
                t::bottom_bar::handshaking_repo(locale.get()).to_string(),
            ),
            SyncStatusKind::SnapshotLoading => (
                "bg-blue-500",
                t::bottom_bar::snapshot_loading(locale.get()).to_string(),
            ),
            SyncStatusKind::Reconnecting => (
                "bg-yellow-500",
                t::bottom_bar::reconnecting(locale.get()).to_string(),
            ),
            SyncStatusKind::NativeSessionPending => (
                "bg-yellow-500",
                t::bottom_bar::native_session_pending(locale.get()).to_string(),
            ),
            SyncStatusKind::NativeReprobeRequired => (
                "bg-yellow-500",
                t::bottom_bar::native_reprobe_required(locale.get()).to_string(),
            ),
            SyncStatusKind::SessionExpired => (
                "bg-amber-500",
                t::bottom_bar::unauthorized(locale.get()).to_string(),
            ),
            SyncStatusKind::NativeBootstrapInvalid => (
                "bg-red-500",
                t::bottom_bar::native_bootstrap_invalid(locale.get()).to_string(),
            ),
            SyncStatusKind::NativeServiceOffline => (
                "bg-red-500",
                t::bottom_bar::native_service_offline(locale.get()).to_string(),
            ),
            SyncStatusKind::Offline => (
                "bg-red-500",
                t::bottom_bar::offline(locale.get()).to_string(),
            ),
        };
        view! {
            <div class="flex items-center gap-1.5">
                <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
                <span class="text-[11px] text-secondary font-medium">{text}</span>
            </div>
        }
        .into_any()
    }
}

/// Loading progress bar (hidden when `load_state == "ready"`).
#[component]
pub fn LoadStatus(
    load_state: ReadSignal<String>,
    load_progress: ReadSignal<(usize, usize)>,
    load_eta_ms: ReadSignal<u64>,
    is_narrow: ReadSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    move || {
        if load_state.get() == "ready" {
            return view! {}.into_any();
        }
        let (done, total) = load_progress.get();
        let eta_ms = load_eta_ms.get();
        let text = if total > 0 {
            if eta_ms > 0 && !is_narrow.get() {
                format!(
                    "{} {}/{} (~{}ms)",
                    t::bottom_bar::loading(locale.get()),
                    done,
                    total,
                    eta_ms,
                )
            } else {
                format!("L {}/{}", done, total)
            }
        } else {
            t::bottom_bar::loading(locale.get()).to_string()
        };
        view! { <div class="text-[10px] text-muted font-mono">{text}</div> }.into_any()
    }
}
