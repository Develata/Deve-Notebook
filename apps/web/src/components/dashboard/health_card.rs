// apps/web/src/components/dashboard/health_card.rs
//! # Health Card (健康状态卡片)
//!
//! 显示 CPU 使用率、内存占用和服务器运行时间。

use crate::hooks::use_core::SystemMetricsData;
use leptos::prelude::*;

/// 将秒数格式化为 "Xd Xh Xm" 可读格式
fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

#[component]
pub fn HealthCard(metrics: SystemMetricsData) -> impl IntoView {
    let cpu_color = if metrics.cpu_usage_percent > 80.0 {
        "text-red-500"
    } else if metrics.cpu_usage_percent > 50.0 {
        "text-yellow-500"
    } else {
        "text-green-500"
    };

    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">"Server Health"</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"CPU"</span>
                    <span class={format!("text-sm font-mono font-semibold {}", cpu_color)}>
                        {format!("{:.1}%", metrics.cpu_usage_percent)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"Memory"</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {format!("{} MB", metrics.memory_used_mb)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">"Uptime"</span>
                    <span class="text-sm font-mono text-primary">
                        {format_uptime(metrics.uptime_secs)}
                    </span>
                </div>
            </div>
        </div>
    }
}
