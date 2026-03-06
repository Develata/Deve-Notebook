// apps/web/src/components/dashboard/sync_card.rs
//! # Sync Card (同步状态卡片)
//!
//! 显示已连接的 Peer 数量和已处理的操作总数。

use crate::hooks::use_core::SystemMetricsData;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn SyncCard(metrics: SystemMetricsData) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::sync_status(locale.get())}</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::connected_peers(locale.get())}</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.active_connections.to_string()}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::ops_processed(locale.get())}</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.ops_processed.to_string()}
                    </span>
                </div>
            </div>
        </div>
    }
}
