// apps/web/src/components/mobile_layout/footer_status.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
//! # Mobile Footer — Status & Load Indicators

use crate::hooks::use_core::CoreState;
use crate::runtime::document::pending::{
    PendingLocalEdits, PendingScope, pending_count_for_doc_in_scope,
};
use crate::hooks::use_core::status_summary::{SyncStatusInput, SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

fn pending_ack_count_for_current_scope(
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

/// Connection status indicator (green/yellow/red dot + text).
#[component]
pub fn StatusView(core: CoreState, locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let current_doc = core.current_doc.get();
        let current_repo_id = core.current_repo_id.get();
        let current_scope_nonce = core.current_scope_nonce.get();
        let pending_ack_count = pending_ack_count_for_current_scope(
            &core.pending_local_edits.get(),
            current_doc,
            current_repo_id.as_deref(),
            current_scope_nonce,
        );
        let handshake_ready = core.handshake_ready.get();
        let readiness = core.ws.native_runtime_readiness_for(
            current_repo_id.as_deref(),
            Some(current_scope_nonce),
            handshake_ready,
        );
        let summary = derive_sync_status(SyncStatusInput {
            connection_status: core.ws.status.get(),
            load_state: &core.load_state.get(),
            remote_branch_active: core.active_branch.get().is_some(),
            degraded_storage: core.is_spectator.get() && core.active_branch.get().is_none(),
            node_role_probe_failed: core.ws.node_role_probe_failed.get(),
            node_role_readable: readiness.node_role_readable,
            handshake_ready: readiness.repo_handshake_complete,
            writer_ready: readiness.writer_ready,
            current_repo_id: current_repo_id.as_deref(),
            current_repo_name: core.current_repo.get().as_deref(),
            pending_repo_switch: core.pending_repo_switch.get().as_deref(),
            pending_branch_switch: core.pending_branch_switch.get().is_some(),
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
                <Show when=move || matches!(summary.kind, SyncStatusKind::PeerNotRegistered)>
                    <button
                        type="button"
                        class="text-[11px] text-accent underline underline-offset-2"
                        data-deve-peer-registration-retry="mobile"
                        aria-label={t::bottom_bar::retry_peer_registration(locale.get())}
                        on:click=move |_| core.on_retry_peer_registration.run(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::document::pending::{PendingLocalEditInput, push_pending_edit};
    use deve_core::models::{Op, RepoId};

    fn push_insert(
        pending: &mut PendingLocalEdits,
        repo_id: RepoId,
        doc_id: DocId,
        scope_nonce: u64,
        client_op_id: u64,
    ) {
        push_pending_edit(
            pending,
            PendingLocalEditInput {
                repo_id,
                doc_id,
                scope_nonce,
                client_id: 1,
                client_op_id,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "x".into(),
                },
            },
        );
    }

    #[test]
    fn pending_ack_count_uses_current_repo_scope() {
        let current_repo = RepoId::from_u128(1);
        let other_repo = RepoId::from_u128(2);
        let current_doc = DocId::from_u128(10);
        let mut pending = PendingLocalEdits::new();

        push_insert(&mut pending, current_repo, current_doc, 7, 1);
        push_insert(&mut pending, current_repo, current_doc, 8, 2);
        push_insert(&mut pending, other_repo, current_doc, 7, 3);

        assert_eq!(
            pending_ack_count_for_current_scope(
                &pending,
                Some(current_doc),
                Some(&current_repo.to_string()),
                7,
            ),
            1
        );
        assert_eq!(
            pending_ack_count_for_current_scope(&pending, Some(current_doc), None, 7),
            0
        );
        assert_eq!(
            pending_ack_count_for_current_scope(&pending, Some(current_doc), Some("not-a-uuid"), 7),
            0
        );
    }
}
