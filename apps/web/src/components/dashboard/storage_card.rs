// apps/web/src/components/dashboard/storage_card.rs
//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 18_release#runtime-observability
//!
//! # Storage Card (存储状态卡片)
//!
//! 显示数据库大小和文档数量。

use crate::hooks::use_core::SystemMetricsData;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// 将字节数格式化为人类可读单位 (KB / MB / GB)
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

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
                        {format_bytes(metrics.db_size_bytes)}
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
