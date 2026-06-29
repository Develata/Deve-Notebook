// apps/web/src/components/dashboard/storage_card.rs
//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 18_release#runtime-observability
//!   - 13_i18n#i18n-facade-contract
//!
//! # Storage Card (存储状态卡片)
//!
//! 显示数据库大小和文档数量。

use crate::hooks::use_core::SystemMetricsData;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn StorageCard(metrics: SystemMetricsData) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        <div
            class="bg-panel rounded-lg border border-default p-4"
            data-deve-dashboard-card="storage"
            data-deve-dashboard-storage-source="ws-system-metrics"
            data-deve-dashboard-storage-db-size-bytes=metrics.db_size_bytes.to_string()
            data-deve-dashboard-storage-doc-count=metrics.doc_count.to_string()
        >
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::storage(locale.get())}</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::db_size(locale.get())}</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {move || t::dashboard::format_bytes(locale.get(), metrics.db_size_bytes)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::documents(locale.get())}</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.doc_count.to_string()}
                    </span>
                </div>
            </div>
        </div>
    }
}
