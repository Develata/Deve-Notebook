// apps/web/src/components/dashboard/mod.rs
//! # Dashboard (仪表盘)
//!
//! 当没有文档被选中时，在主内容区显示服务器运行指标。
//!
//! **Invariant**: 所有指标仅存于 RAM 信号中，不持久化到 IndexedDB。
//! 当 WebSocket 断开时，指标冻结并显示 "Waiting for server..." 提示。

mod actions_card;
mod health_card;
mod runtime_card;
mod storage_card;
mod sync_card;

use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::DashboardContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use self::actions_card::ActionsCard;
use self::health_card::HealthCard;
use self::runtime_card::RuntimeCard;
use self::storage_card::StorageCard;
use self::sync_card::SyncCard;

#[component]
pub fn Dashboard() -> impl IntoView {
    let ctx = expect_context::<DashboardContext>();
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let repo_name = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "No repo selected".to_string())
    });
    let doc_count = Signal::derive(move || core.docs.get().len());

    view! {
        <div class="w-full h-full min-h-0 overflow-auto p-6">
            <div class="mx-auto flex min-h-full w-full max-w-xl items-center justify-center">
                <div class="w-full space-y-4">
                    <div class="rounded-2xl border border-default bg-app px-6 py-6 shadow-sm">
                        <div class="text-center">
                            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-muted">
                                {move || t::dashboard::title(locale.get())}
                            </div>
                            <h2 class="mt-2 text-xl font-bold text-primary">
                                {move || repo_name.get()}
                            </h2>
                            <p class="mt-2 text-sm text-secondary">
                                {move || format!("{} docs in current repo", doc_count.get())}
                            </p>
                            <p class="mt-3 text-sm text-muted">
                                {move || t::dashboard::waiting_metrics(locale.get())}
                            </p>
                        </div>
                    </div>
                    {move || match ctx.metrics.get() {
                        Some(m) => view! {
                            <div class="space-y-3">
                                <RuntimeCard runtime_summary=core.ws.node_role />
                                <HealthCard metrics=m.clone() />
                                <SyncCard metrics=m.clone() />
                                <StorageCard metrics=m.clone() />
                                <ActionsCard />
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="space-y-3">
                                <RuntimeCard runtime_summary=core.ws.node_role />
                                <ActionsCard />
                            </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}
