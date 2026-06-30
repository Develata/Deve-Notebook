// apps/web/src/components/dashboard/health_card.rs
//! plan_ref:
//!   - 18_release#runtime-observability
//!   - 13_i18n#i18n-facade-contract
//!
//! # Health Card (健康状态卡片)
//!
//! 显示 CPU 使用率、内存占用和服务器运行时间。

use crate::hooks::use_core::SystemMetricsData;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn HealthCard(metrics: SystemMetricsData) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    let cpu_color = if metrics.cpu_usage_percent > 80.0 {
        "text-red-500"
    } else if metrics.cpu_usage_percent > 50.0 {
        "text-yellow-500"
    } else {
        "text-green-500"
    };

    view! {
        <div
            class="bg-panel rounded-lg border border-default p-4"
            data-deve-dashboard-card="system-health"
            data-deve-dashboard-health-source="ws-system-metrics"
            data-deve-dashboard-health-sample=metrics.sample_seq.to_string()
            data-deve-dashboard-health-uptime-secs=metrics.uptime_secs.to_string()
        >
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::server_health(locale.get())}</h3>
            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::cpu(locale.get())}</span>
                    <span class={format!("text-sm font-mono font-semibold {}", cpu_color)}>
                        {move || t::dashboard::format_cpu_percent(locale.get(), metrics.cpu_usage_percent)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::memory(locale.get())}</span>
                    <span class="text-sm font-mono font-semibold text-primary">
                        {move || t::dashboard::format_memory_mb(locale.get(), metrics.memory_used_mb)}
                    </span>
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-muted">{move || t::dashboard::uptime(locale.get())}</span>
                    <span class="text-sm font-mono text-primary">
                        {move || t::dashboard::format_uptime(locale.get(), metrics.uptime_secs)}
                    </span>
                </div>
            </div>
        </div>
    }
}
