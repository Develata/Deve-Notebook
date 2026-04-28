//! plan_ref:
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
        derive_sync_status(
            core.ws.status.get(),
            &core.load_state.get(),
            core.active_branch.get().is_some(),
            core.is_spectator.get() && core.active_branch.get().is_none(),
            core.handshake_ready.get(),
            core.ws.writer_ready_for(
                core.current_repo_id.get().as_deref(),
                Some(core.current_scope_nonce.get()),
            ),
            core.current_repo_id.get().as_deref(),
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
        SyncStatusKind::HandshakingRepo | SyncStatusKind::Reconnecting => "bg-yellow-500",
        SyncStatusKind::SnapshotLoading => "bg-blue-500",
        SyncStatusKind::SessionExpired => "bg-amber-500",
        SyncStatusKind::Offline => "bg-red-500",
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
