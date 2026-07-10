// apps/cli/src/server/metrics.rs
//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! # 系统指标采集 (System Metrics Collection)
//!
//! 轻量级系统指标采集，无外部依赖。
//!
//! **约束**: 768 MB 内存 VPS，禁止常驻采集线程。
//! 采用定时快照策略：每 5 秒采集一次瞬时值并广播。
//!
//! **平台支持**:
//! - Linux: 优先读取当前可见 cgroup hierarchy，必要时回退 `/proc`
//! - 其他平台: 安全降级 (CPU=0, 内存=0)

use crate::server::AppState;
use anyhow::Context;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[cfg(any(target_os = "linux", test))]
mod cgroup;

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
    let db_size = db_file_size(state.repo.ledger_dir()).unwrap_or_else(|err| {
        tracing::warn!("Failed to collect DB size metrics: {err}");
        0
    });
    let doc_count = local_doc_count(state).unwrap_or_else(|err| {
        tracing::warn!("Failed to collect doc count metrics: {err}");
        0
    });
    (db_size, doc_count)
}

fn local_doc_count(state: &AppState) -> anyhow::Result<u32> {
    // Invariant: doc_count 统计所有本地 repo 的文档总数，而不是默认主库的投影。
    let mut total = 0u32;
    for repo_name in state.repo.list_local_repo_names_for_execution()? {
        let count = state.repo.list_local_docs(Some(&repo_name))?.len();
        total = total.saturating_add(count as u32);
    }
    Ok(total)
}

/// 计算 ledger 目录下所有 .redb 文件总大小
fn db_file_size(ledger_dir: &std::path::Path) -> anyhow::Result<u64> {
    let local_dir = ledger_dir.join("local");
    match local_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(0),
        Err(err) => {
            return Err(err).context(format!(
                "Failed to stat local metrics directory: {:?}",
                local_dir
            ));
        }
    }

    let mut total = 0u64;
    for entry in std::fs::read_dir(local_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "redb") {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
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
mod linux;

#[cfg(test)]
mod tests {
    use super::db_file_size;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn db_file_size_counts_only_redb_files() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let local = dir.path().join("local");
        fs::create_dir_all(&local)?;
        fs::write(local.join("a.redb"), vec![0u8; 3])?;
        fs::write(local.join("b.redb"), vec![0u8; 5])?;
        fs::write(local.join("note.txt"), vec![0u8; 99])?;

        assert_eq!(db_file_size(dir.path())?, 8);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn db_file_size_fails_closed_on_unstatable_local_dir() {
        let dir = tempdir().expect("tempdir");
        let local = dir.path().join("local");
        fs::create_dir_all(&local).expect("mkdir");
        let original = fs::metadata(&local).expect("metadata").permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        fs::set_permissions(&local, blocked).expect("chmod 000");

        let err = db_file_size(dir.path()).expect_err("unstatable local dir must fail closed");

        fs::set_permissions(&local, original).expect("restore perms");
        assert!(
            err.to_string()
                .contains("Failed to stat local metrics directory")
                || err.to_string().contains("Permission denied")
        );
    }
}
