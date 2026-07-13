// apps/web/src/components/mobile_layout/footer_status.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
//! # Mobile Footer — Status & Load Indicators

use super::footer_read::read_footer_signal;
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::status_summary::{SyncStatusInput, SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use crate::runtime::document::pending::{
    PendingLocalEdits, PendingScope, pending_count_for_doc_in_scope,
};
use crate::runtime::domain::LoadPhase;
use crate::runtime::{
    rendering_client::RenderingClient, scope_client::ScopeClient, session_client::SessionClient,
};
use deve_core::models::DocId;
use leptos::prelude::*;

pub(crate) fn pending_ack_count_for_current_scope(
    pending: &PendingLocalEdits,
    current_doc: Option<DocId>,
    current_repo_id: Option<&str>,
    current_scope_nonce: u64,
) -> usize {
    current_doc
        .and_then(|doc_id| {
            PendingScope::from_repo_id_str(current_repo_id, current_scope_nonce)
                .map(|scope| pending_count_for_doc_in_scope(pending, doc_id, scope))
        })
        .unwrap_or_default()
}

fn mobile_load_status_text(
    locale: Locale,
    done: usize,
    total: usize,
    eta_ms: u64,
    is_narrow: bool,
) -> String {
    if total == 0 {
        return t::bottom_bar::loading(locale).to_string();
    }

    if is_narrow {
        t::bottom_bar::loading_progress_compact(locale, done, total)
    } else {
        t::bottom_bar::loading_progress(locale, done, total, eta_ms)
    }
}

/// Connection status indicator (green/yellow/red dot + text).
#[component]
pub fn StatusView(locale: RwSignal<Locale>, pending_ack_count: Memo<usize>) -> impl IntoView {
    let session = expect_context::<SessionClient>();
    let scope = expect_context::<ScopeClient>();
    let editor = expect_context::<EditorContext>();
    let rendering = expect_context::<RenderingClient>();

    move || {
        let current_repo_id = scope.current_repo_id.get();
        let current_scope_nonce = scope.current_scope_nonce.get();
        let pending_ack_count = pending_ack_count.get();
        let handshake_ready = session.handshake_ready.get();
        let readiness = session.ws.native_runtime_readiness_for(
            current_repo_id.as_deref(),
            Some(current_scope_nonce),
            handshake_ready,
        );
        let summary = derive_sync_status(SyncStatusInput {
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
        });
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
            SyncStatusKind::PeerNotRegistered => (
                "bg-slate-400",
                t::bottom_bar::peer_not_registered(locale.get()).to_string(),
            ),
            SyncStatusKind::HandshakingRepo => (
                "bg-yellow-500",
                t::bottom_bar::handshaking_repo(locale.get()).to_string(),
            ),
            SyncStatusKind::SnapshotLoading => (
                "bg-blue-500",
                t::bottom_bar::snapshot_loading(locale.get()).to_string(),
            ),
            SyncStatusKind::EditorSyncError => (
                "bg-deleted",
                t::bottom_bar::editor_sync_error(locale.get()).to_string(),
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
            <div
                class="flex items-center gap-1.5"
                data-deve-sync-status=summary.kind.marker()
                data-deve-pending-ack-count=summary.pending_ack_count.to_string()
            >
                <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
                <span class="text-[11px] text-secondary font-medium">{text}</span>
                <Show when=move || matches!(summary.kind, SyncStatusKind::PeerNotRegistered)>
                    <button
                        type="button"
                        class="text-[11px] text-accent underline underline-offset-2"
                        data-deve-peer-registration-retry="mobile"
                        aria-label={t::bottom_bar::retry_peer_registration(locale.get())}
                        on:click=move |_| session.on_retry_peer_registration.run(())
                    >
                        {t::bottom_bar::retry_peer_registration(locale.get())}
                    </button>
                </Show>
            </div>
        }
        .into_any()
    }
}

/// Loading progress bar (hidden when `load_state == "ready"`).
#[component]
pub fn LoadStatus(
    load_state: ReadSignal<LoadPhase>,
    load_progress: ReadSignal<(usize, usize)>,
    load_eta_ms: ReadSignal<u64>,
    is_narrow: ReadSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    move || {
        let state = read_footer_signal(load_state, LoadPhase::Ready);
        if state.is_ready() {
            return view! {}.into_any();
        }
        if state == LoadPhase::Error {
            return view! {
                <div class="text-[10px] text-deleted font-mono">
                    {t::bottom_bar::editor_sync_error(locale.get())}
                </div>
            }
            .into_any();
        }
        let (done, total) = read_footer_signal(load_progress, (0, 0));
        let eta_ms = read_footer_signal(load_eta_ms, 0);
        let text = mobile_load_status_text(
            locale.get(),
            done,
            total,
            eta_ms,
            read_footer_signal(is_narrow, false),
        );
        view! { <div class="text-[10px] text-muted font-mono">{text}</div> }.into_any()
    }
}

#[cfg(test)]
mod tests;
