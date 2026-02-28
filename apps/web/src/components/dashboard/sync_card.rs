// apps/web/src/components/dashboard/sync_card.rs
//! # Sync Card (同步状态卡片)
//!
//! 显示已连接的 Peer 数量和已处理的操作总数。

use crate::hooks::use_core::SystemMetricsData;
use leptos::prelude::*;

#[component]
pub fn SyncCard(metrics: SystemMetricsData) -> impl IntoView {
    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">"Sync Status"</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"Connected Peers"</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.active_connections.to_string()}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"Ops Processed"</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {metrics.ops_processed.to_string()}
                    </span>
                </div>
            </div>
        </div>
    }
}
