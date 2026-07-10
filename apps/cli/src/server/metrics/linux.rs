//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! Linux cgroup-aware runtime resource metrics with host `/proc` fallback.

use super::cgroup::{self, CgroupSources};
use std::fs;
use std::time::{Duration, Instant};

const MEMINFO: &str = "/proc/meminfo";
const CPU_STAT: &str = "/proc/stat";
const SAMPLE_WINDOW: Duration = Duration::from_millis(100);

pub(super) fn memory_used_mb() -> u64 {
    let meminfo = fs::read_to_string(MEMINFO).unwrap_or_default();
    let cgroup_usage = CgroupSources::detect().and_then(|sources| sources.memory_usage_bytes());
    cgroup::memory_used_mb(cgroup_usage, &meminfo)
}

pub(super) fn cpu_usage() -> f32 {
    if let Some(source) = CgroupSources::detect().and_then(|sources| sources.cpu_usage_source())
        && let Some(usage_start) = source.read_usage_micros()
    {
        let started = Instant::now();
        std::thread::sleep(SAMPLE_WINDOW);
        if let Some(usage_end) = source.read_usage_micros() {
            return cgroup::cpu_usage_percent(
                usage_start,
                usage_end,
                started.elapsed(),
                source.capacity_cores(),
            );
        }
    }
    host_cpu_usage()
}

fn host_cpu_usage() -> f32 {
    let Some(start) = read_host_cpu_stat() else {
        return 0.0;
    };
    std::thread::sleep(SAMPLE_WINDOW);
    let Some(end) = read_host_cpu_stat() else {
        return 0.0;
    };
    let total_delta = end.total.saturating_sub(start.total);
    let idle_delta = end.idle.saturating_sub(start.idle);
    if total_delta == 0 {
        return 0.0;
    }
    ((total_delta - idle_delta) as f32 / total_delta as f32) * 100.0
}

struct HostCpuStat {
    total: u64,
    idle: u64,
}

fn read_host_cpu_stat() -> Option<HostCpuStat> {
    let content = fs::read_to_string(CPU_STAT).ok()?;
    let line = content.lines().next()?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if values.len() < 4 {
        return None;
    }
    Some(HostCpuStat {
        total: values.iter().sum(),
        idle: values[3],
    })
}
