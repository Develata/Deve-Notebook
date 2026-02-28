// apps/web/src/components/dashboard/storage_card.rs
//! # Storage Card (存储状态卡片)
//!
//! 显示数据库大小和文档数量。

use crate::hooks::use_core::SystemMetricsData;
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
    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">"Storage"</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"DB Size"</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {format_bytes(metrics.db_size_bytes)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"Documents"</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.doc_count.to_string()}
                    </span>
                </div>
            </div>
        </div>
    }
}
