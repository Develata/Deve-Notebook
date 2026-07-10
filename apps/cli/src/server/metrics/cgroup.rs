//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! Pure cgroup metric parsing, normalization, and source selection.

mod layout;
#[cfg(test)]
mod layout_tests;
mod membership;

use std::time::Duration;

#[cfg(target_os = "linux")]
pub(super) use layout::CgroupSources;

const BYTES_PER_MIB: u64 = 1024 * 1024;

pub(super) fn memory_used_mb(cgroup_usage_bytes: Option<u64>, host_meminfo: &str) -> u64 {
    cgroup_usage_bytes
        .map(|bytes| bytes / BYTES_PER_MIB)
        .unwrap_or_else(|| host_memory_used_mb(host_meminfo))
}

fn host_memory_used_mb(meminfo: &str) -> u64 {
    let mut total = 0u64;
    let mut available = 0u64;
    for line in meminfo.lines() {
        if let Some(value) = parse_meminfo_kb(line, "MemTotal:") {
            total = value;
        } else if let Some(value) = parse_meminfo_kb(line, "MemAvailable:") {
            available = value;
        }
    }
    total.saturating_sub(available) / 1024
}

fn parse_meminfo_kb(line: &str, prefix: &str) -> Option<u64> {
    line.strip_prefix(prefix)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

pub(super) fn cpu_usage_percent(
    usage_start_micros: u64,
    usage_end_micros: u64,
    elapsed: Duration,
    capacity_cores: f64,
) -> f32 {
    if !capacity_cores.is_finite() || capacity_cores <= 0.0 {
        return 0.0;
    }
    let elapsed_micros = elapsed.as_micros() as f64;
    if elapsed_micros <= 0.0 {
        return 0.0;
    }
    let usage_delta = usage_end_micros.saturating_sub(usage_start_micros) as f64;
    ((usage_delta / elapsed_micros / capacity_cores) * 100.0).clamp(0.0, 100.0) as f32
}

pub(super) fn parse_cpu_stat_usage_micros(cpu_stat: &str) -> Option<u64> {
    cpu_stat.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next()? == "usage_usec")
            .then(|| fields.next()?.parse().ok())
            .flatten()
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum CpuQuota {
    Unlimited,
    Limited(f64),
}

pub(super) fn parse_cpu_max(cpu_max: &str) -> Option<CpuQuota> {
    let mut fields = cpu_max.split_whitespace();
    let quota = fields.next()?;
    let period = fields.next()?.parse::<u64>().ok()?;
    if fields.next().is_some() || period == 0 {
        return None;
    }
    if quota == "max" {
        return Some(CpuQuota::Unlimited);
    }
    let quota = quota.parse::<u64>().ok()?;
    (quota > 0).then_some(CpuQuota::Limited(quota as f64 / period as f64))
}

pub(super) fn parse_cpuset_capacity(cpuset: &str) -> Option<f64> {
    let mut count = 0u64;
    for segment in cpuset.trim().split(',').filter(|part| !part.is_empty()) {
        let mut bounds = segment.split('-');
        let start = bounds.next()?.parse::<u64>().ok()?;
        let end = bounds
            .next()
            .map(str::parse::<u64>)
            .transpose()
            .ok()?
            .unwrap_or(start);
        if bounds.next().is_some() || end < start {
            return None;
        }
        let width = end.checked_sub(start)?.checked_add(1)?;
        count = count.checked_add(width)?;
    }
    (count > 0).then_some(count as f64)
}

pub(super) fn parse_v1_cpu_quota(quota: &str, period: &str) -> Option<CpuQuota> {
    let quota = quota.trim().parse::<i64>().ok()?;
    let period = period.trim().parse::<u64>().ok()?;
    if period == 0 || quota == 0 || quota < -1 {
        return None;
    }
    if quota == -1 {
        return Some(CpuQuota::Unlimited);
    }
    Some(CpuQuota::Limited(quota as f64 / period as f64))
}

pub(super) fn effective_cpu_capacity(quota: CpuQuota, cpuset_cores: f64) -> f64 {
    match quota {
        CpuQuota::Unlimited => cpuset_cores,
        CpuQuota::Limited(quota_cores) => quota_cores.min(cpuset_cores),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CpuQuota, cpu_usage_percent, effective_cpu_capacity, parse_cpu_max,
        parse_cpu_stat_usage_micros, parse_cpuset_capacity, parse_v1_cpu_quota,
    };
    use std::time::Duration;

    #[test]
    fn cgroup_cpu_usage_is_normalized_by_effective_capacity() {
        assert_eq!(
            cpu_usage_percent(1_000_000, 1_100_000, Duration::from_millis(100), 4.0),
            25.0
        );
        assert_eq!(effective_cpu_capacity(CpuQuota::Limited(4.0), 2.0), 2.0);
        assert_eq!(effective_cpu_capacity(CpuQuota::Unlimited, 2.0), 2.0);
    }

    #[test]
    fn cgroup_cpu_parsers_reject_invalid_kernel_shapes() {
        assert_eq!(
            parse_cpu_stat_usage_micros("usage_usec 12345\nuser_usec 10000\nsystem_usec 2345\n"),
            Some(12_345)
        );
        assert_eq!(
            parse_cpu_max("200000 100000\n"),
            Some(CpuQuota::Limited(2.0))
        );
        assert_eq!(parse_cpu_max("max 100000\n"), Some(CpuQuota::Unlimited));
        assert_eq!(parse_cpu_max("1 NaN\n"), None);
        assert_eq!(parse_cpuset_capacity("0-3,6,8-9\n"), Some(7.0));
        assert_eq!(parse_cpuset_capacity(&format!("0-{}", u64::MAX)), None);
        assert_eq!(
            parse_v1_cpu_quota("150000\n", "100000\n"),
            Some(CpuQuota::Limited(1.5))
        );
        assert_eq!(
            parse_v1_cpu_quota("-1\n", "100000\n"),
            Some(CpuQuota::Unlimited)
        );
    }
}
