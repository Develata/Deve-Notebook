use leptos::prelude::*;

/// 系统指标快照 (来自服务端 SystemMetrics 推送)
///
/// Invariant:
/// - 仅存于 RAM 信号中, 不持久化到 IndexedDB。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemMetricsData {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u64,
    pub active_connections: u32,
    pub ops_processed: u64,
    pub uptime_secs: u64,
    pub db_size_bytes: u64,
    pub doc_count: u32,
}

/// 仪表盘上下文 (Dashboard 组件消费)
#[derive(Clone)]
pub struct DashboardContext {
    pub metrics: ReadSignal<Option<SystemMetricsData>>,
}
