// apps/cli/src/server/metrics.rs
//! # 系统指标采集 (System Metrics Collection)
//!
//! 轻量级系统指标采集，无外部依赖。
//!
//! **约束**: 768 MB 内存 VPS，禁止常驻采集线程。
//! 采用定时快照策略：每 5 秒采集一次瞬时值并广播。
//!
//! **平台支持**:
//! - Linux: 解析 `/proc/meminfo` + `/proc/stat`
//! - 其他平台: 安全降级 (CPU=0, 内存=0)

use crate::server::AppState;
use deve_core::ledger::traits::Repository;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// 全局操作计数器 (Handler 中调用 `increment_ops()` 累加)
static OPS_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 服务器启动时间 (OnceLock: 初始化一次，无 unsafe)
static START_TIME: OnceLock<Instant> = OnceLock::new();

/// 初始化启动时间 (在 `start_server` 中调用一次)
pub fn init_start_time() {
    let _ = START_TIME.set(Instant::now());
}

/// 递增操作计数 (供 Handler 调用)
pub fn increment_ops() {
    OPS_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// 采集瞬时系统指标
///
/// **Invariant**: 不分配堆内存（除字符串解析的临时 buffer）
pub fn collect(state: &AppState) -> ServerMessage {
    let uptime_secs = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);
    let active_connections = state.tx.receiver_count() as u32;
    let ops_processed = OPS_COUNTER.load(Ordering::Relaxed);

    let (cpu_usage_percent, memory_used_mb) = platform_metrics();

    let (db_size_bytes, doc_count) = storage_metrics(state);

    ServerMessage::SystemMetrics {
        cpu_usage_percent,
        memory_used_mb,
        active_connections,
        ops_processed,
        uptime_secs,
        db_size_bytes,
        doc_count,
    }
}

/// 启动指标广播任务 (每 5 秒)
pub fn spawn_broadcaster(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let msg = collect(&state);
            let _ = state.tx.send(msg);
        }
    });
}

/// 存储指标: DB 文件大小 + 文档数
fn storage_metrics(state: &AppState) -> (u64, u32) {
    let db_size = db_file_size(&state.vault_path);
    let doc_count = state.repo.list_docs().map(|v| v.len() as u32).unwrap_or(0);
    (db_size, doc_count)
}

/// 计算 ledger 目录下所有 .redb 文件总大小
fn db_file_size(vault_path: &std::path::Path) -> u64 {
    let local_dir = vault_path.join(".deve").join("ledger").join("local");
    std::fs::read_dir(local_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "redb"))
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// 平台相关的 CPU / 内存指标
#[cfg(target_os = "linux")]
fn platform_metrics() -> (f32, u64) {
    (linux::cpu_usage(), linux::memory_used_mb())
}

#[cfg(not(target_os = "linux"))]
fn platform_metrics() -> (f32, u64) {
    (0.0, 0)
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;

    /// 读取 /proc/meminfo 计算已用内存 (MB)
    pub fn memory_used_mb() -> u64 {
        let Ok(content) = fs::read_to_string("/proc/meminfo") else {
            return 0;
        };
        let mut total = 0u64;
        let mut available = 0u64;
        for line in content.lines().take(8) {
            if let Some(val) = parse_meminfo_kb(line, "MemTotal:") {
                total = val;
            } else if let Some(val) = parse_meminfo_kb(line, "MemAvailable:") {
                available = val;
            }
        }
        total.saturating_sub(available) / 1024
    }

    fn parse_meminfo_kb(line: &str, prefix: &str) -> Option<u64> {
        let rest = line.strip_prefix(prefix)?;
        rest.trim().split_whitespace().next()?.parse().ok()
    }

    /// 瞬时 CPU 使用率 (/proc/stat 两次采样, 间隔 100ms)
    pub fn cpu_usage() -> f32 {
        let Some(s1) = read_cpu_stat() else {
            return 0.0;
        };
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(s2) = read_cpu_stat() else {
            return 0.0;
        };
        let total_d = s2.total.saturating_sub(s1.total);
        let idle_d = s2.idle.saturating_sub(s1.idle);
        if total_d == 0 {
            return 0.0;
        }
        ((total_d - idle_d) as f32 / total_d as f32) * 100.0
    }

    struct CpuStat {
        total: u64,
        idle: u64,
    }

    fn read_cpu_stat() -> Option<CpuStat> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let line = content.lines().next()?;
        let vals: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();
        if vals.len() < 4 {
            return None;
        }
        let idle = vals[3];
        let total: u64 = vals.iter().sum();
        Some(CpuStat { total, idle })
    }
}
