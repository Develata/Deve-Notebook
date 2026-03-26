// apps\web\src\components
//! # BottomBar 组件 (BottomBar Component)
//!
//! 底部状态栏，显示分支切换器、连接状态和编辑器统计信息 (字数、行数、字符数)。

use crate::components::branch_switcher::BranchSwitcher;
use crate::editor::EditorStats;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::status_summary::{SyncStatusKind, derive_sync_status};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn BottomBar(core: CoreState) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let stats = core.stats;
    let max_ver = core.doc_version;
    let curr_ver = core.playback_version;
    let set_ver = core.set_playback_version;
    let current_doc = core.current_doc;
    let load_state = core.load_state;
    let load_progress = core.load_progress;
    let load_eta_ms = core.load_eta_ms;
    let displayed_stats = Signal::derive(move || {
        if current_doc.get().is_some() {
            stats.get()
        } else {
            EditorStats::default()
        }
    });
    let displayed_max_ver =
        Signal::derive(move || if current_doc.get().is_some() { max_ver.get() } else { 0 });
    let displayed_curr_ver =
        Signal::derive(move || if current_doc.get().is_some() { curr_ver.get() } else { 0 });

    let status_view = move || {
        let current_doc = core.current_doc.get();
        let pending_ack_count = current_doc
            .and_then(|doc_id| core.pending_local_edits.get().get(&doc_id).map(Vec::len))
            .unwrap_or_default();
        let summary = derive_sync_status(
            core.ws.status.get(),
            &load_state.get(),
            core.active_branch.get().is_some(),
            core.is_spectator.get() && core.active_branch.get().is_none(),
            core.handshake_ready.get(),
            core.ws
                .writer_ready_for(core.current_repo_id.get().as_deref()),
            core.current_repo_id.get().as_deref(),
            core.current_repo.get().as_deref(),
            core.pending_repo_switch.get().as_deref(),
            core.pending_branch_switch.get().is_some(),
            pending_ack_count,
        );
        let repo_label = if matches!(summary.kind, SyncStatusKind::HandshakingRepo) {
            summary.repo_name.clone().unwrap_or_default()
        } else {
            String::new()
        };
        let show_repo_label = !repo_label.is_empty();
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
            SyncStatusKind::SessionExpired => (
                "bg-amber-500",
                t::bottom_bar::unauthorized(locale.get()).to_string(),
            ),
            SyncStatusKind::Offline => (
                "bg-red-500",
                t::bottom_bar::offline(locale.get()).to_string(),
            ),
        };

        view! {
             <div class="flex items-center gap-2 min-w-0">
                <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
                <span class="text-xs text-secondary font-medium">{text}</span>
                <Show when=move || show_repo_label>
                    <span class="text-[10px] text-muted font-mono truncate">
                        {repo_label.clone()}
                    </span>
                </Show>
            </div>
        }
    };

    let time_travel_view = move || {
        view! {
            <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 flex items-center gap-2">
                <span class="text-[10px] text-muted font-mono">
                    {move || format!("v{}/{}", displayed_curr_ver.get(), displayed_max_ver.get())}
                </span>
                <input
                    name="time-travel"
                    type="range"
                    min="0"
                    max=move || displayed_max_ver.get().to_string()
                    value=move || displayed_curr_ver.get().to_string()
                    on:input=move |ev| {
                        let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                        set_ver.set(val);
                    }
                    class="w-32 h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                    title=move || t::bottom_bar::time_travel(locale.get())
                />
            </div>
        }
    };

    let load_status = move || {
        let state = load_state.get();
        if state == "ready" {
            return view! {}.into_any();
        }
        let (done, total) = load_progress.get();
        let eta_ms = load_eta_ms.get();
        let text = if total > 0 {
            t::bottom_bar::loading_progress(locale.get(), done, total, eta_ms)
        } else {
            t::bottom_bar::loading(locale.get()).to_string()
        };
        view! {
            <div class="text-[10px] text-muted font-mono">
                {text}
            </div>
        }
        .into_any()
    };

    view! {
        <footer class="h-8 bg-sidebar border-t border-default flex items-center justify-between px-4 select-none relative">
            // 左侧: 分支切换器 + 系统状态
            <div class="flex items-center gap-3">
                <BranchSwitcher />
                <div class="w-px h-4 bg-active"></div>
                {status_view}
            </div>

            // 中间: Time Travel
            {time_travel_view}

            // 右侧: 编辑器统计

            <div class="flex items-center gap-4 text-xs text-muted">
                {load_status}
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::words(locale.get())}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().words}</span>
                </div>
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::lines(locale.get())}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().lines}</span>
                </div>
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::chars(locale.get())}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().chars}</span>
                </div>
            </div>
        </footer>
    }
}
