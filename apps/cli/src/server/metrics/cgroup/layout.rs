//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! Complete cgroup metric source selection across v2, v1, and visible ancestors.

use super::membership::{Hierarchy, controller_hierarchies, unified_hierarchies, walk_ancestors};
use super::{
    CpuQuota, effective_cpu_capacity, parse_cpu_max, parse_cpu_stat_usage_micros,
    parse_cpuset_capacity, parse_v1_cpu_quota,
};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
const SELF_CGROUP: &str = "/proc/self/cgroup";
#[cfg(target_os = "linux")]
const SELF_MOUNTINFO: &str = "/proc/self/mountinfo";

pub(in crate::server::metrics) struct CgroupSources {
    unified: Vec<Hierarchy>,
    legacy: LegacySources,
}

impl CgroupSources {
    #[cfg(target_os = "linux")]
    pub(in crate::server::metrics) fn detect() -> Option<Self> {
        let membership = fs::read_to_string(SELF_CGROUP).ok()?;
        let mountinfo = fs::read_to_string(SELF_MOUNTINFO).ok()?;
        Some(Self::from_proc(&membership, &mountinfo))
    }

    pub(in crate::server::metrics) fn from_proc(membership: &str, mountinfo: &str) -> Self {
        Self {
            unified: unified_hierarchies(membership, mountinfo),
            legacy: LegacySources::from_proc(membership, mountinfo),
        }
    }

    pub(in crate::server::metrics) fn memory_usage_bytes(&self) -> Option<u64> {
        self.unified
            .iter()
            .find_map(|hierarchy| read_u64(hierarchy.group.join("memory.current")))
            .or_else(|| self.legacy.memory_usage_bytes())
    }

    pub(in crate::server::metrics) fn cpu_usage_source(&self) -> Option<CpuUsageSource> {
        self.unified
            .iter()
            .find_map(unified_cpu_source)
            .or_else(|| self.legacy.cpu_usage_source())
    }
}

pub(in crate::server::metrics) struct CpuUsageSource {
    path: PathBuf,
    format: CpuUsageFormat,
    capacity_cores: f64,
}

impl CpuUsageSource {
    fn new(path: PathBuf, format: CpuUsageFormat, capacity_cores: f64) -> Option<Self> {
        let source = Self {
            path,
            format,
            capacity_cores,
        };
        source.read_usage_micros()?;
        Some(source)
    }

    pub(in crate::server::metrics) fn read_usage_micros(&self) -> Option<u64> {
        let value = fs::read_to_string(&self.path).ok()?;
        match self.format {
            CpuUsageFormat::V2Stat => parse_cpu_stat_usage_micros(&value),
            CpuUsageFormat::V1Nanos => value.trim().parse::<u64>().ok().map(|nanos| nanos / 1000),
        }
    }

    pub(in crate::server::metrics) fn capacity_cores(&self) -> f64 {
        self.capacity_cores
    }
}

#[derive(Clone, Copy)]
enum CpuUsageFormat {
    V2Stat,
    V1Nanos,
}

fn unified_cpu_source(hierarchy: &Hierarchy) -> Option<CpuUsageSource> {
    let quota = strict_ancestor_quota(hierarchy, true, read_v2_quota)?;
    let cpuset = strict_effective_cpuset(hierarchy, &["cpuset.cpus.effective", "cpuset.cpus"])?;
    CpuUsageSource::new(
        hierarchy.group.join("cpu.stat"),
        CpuUsageFormat::V2Stat,
        effective_cpu_capacity(quota, cpuset),
    )
}

struct LegacySources {
    memory: Vec<Hierarchy>,
    cpu: Vec<Hierarchy>,
    cpuacct: Vec<Hierarchy>,
    cpuset: Vec<Hierarchy>,
}

impl LegacySources {
    fn from_proc(membership: &str, mountinfo: &str) -> Self {
        Self {
            memory: controller_hierarchies(membership, mountinfo, "memory"),
            cpu: controller_hierarchies(membership, mountinfo, "cpu"),
            cpuacct: controller_hierarchies(membership, mountinfo, "cpuacct"),
            cpuset: controller_hierarchies(membership, mountinfo, "cpuset"),
        }
    }

    fn memory_usage_bytes(&self) -> Option<u64> {
        self.memory
            .iter()
            .find_map(|hierarchy| read_u64(hierarchy.group.join("memory.usage_in_bytes")))
    }

    fn cpu_usage_source(&self) -> Option<CpuUsageSource> {
        let quota = self
            .cpu
            .iter()
            .find_map(|hierarchy| strict_ancestor_quota(hierarchy, false, read_v1_quota))?;
        let cpuset = self.cpuset.iter().find_map(|hierarchy| {
            strict_effective_cpuset(hierarchy, &["cpuset.effective_cpus", "cpuset.cpus"])
        })?;
        let capacity = effective_cpu_capacity(quota, cpuset);
        self.cpuacct.iter().find_map(|hierarchy| {
            CpuUsageSource::new(
                hierarchy.group.join("cpuacct.usage"),
                CpuUsageFormat::V1Nanos,
                capacity,
            )
        })
    }
}

enum QuotaReadError {
    Missing,
    Invalid,
}

fn read_v2_quota(dir: &Path) -> Result<CpuQuota, QuotaReadError> {
    let value = read_quota_file(dir.join("cpu.max"))?;
    parse_cpu_max(&value).ok_or(QuotaReadError::Invalid)
}

fn read_v1_quota(dir: &Path) -> Result<CpuQuota, QuotaReadError> {
    let quota = read_quota_file(dir.join("cpu.cfs_quota_us"))?;
    let period = read_quota_file(dir.join("cpu.cfs_period_us"))?;
    parse_v1_cpu_quota(&quota, &period).ok_or(QuotaReadError::Invalid)
}

fn read_quota_file(path: PathBuf) -> Result<String, QuotaReadError> {
    fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            QuotaReadError::Missing
        } else {
            QuotaReadError::Invalid
        }
    })
}

fn strict_ancestor_quota(
    hierarchy: &Hierarchy,
    allow_missing_global_root: bool,
    mut read: impl FnMut(&Path) -> Result<CpuQuota, QuotaReadError>,
) -> Option<CpuQuota> {
    let mut valid = true;
    let mut seen = false;
    let mut minimum: Option<f64> = None;
    walk_ancestors(hierarchy, |dir| match read(dir) {
        Ok(CpuQuota::Unlimited) => seen = true,
        Ok(CpuQuota::Limited(value)) => {
            seen = true;
            minimum = Some(minimum.map_or(value, |current| current.min(value)));
        }
        Err(QuotaReadError::Missing)
            if allow_missing_global_root && hierarchy.global_root && dir == hierarchy.mount =>
        {
            seen = true;
        }
        Err(QuotaReadError::Missing | QuotaReadError::Invalid) => valid = false,
    });
    if !valid || !seen {
        return None;
    }
    Some(minimum.map_or(CpuQuota::Unlimited, CpuQuota::Limited))
}

fn strict_effective_cpuset(hierarchy: &Hierarchy, files: &[&str]) -> Option<f64> {
    let mut found = None;
    let mut invalid = false;
    walk_ancestors(hierarchy, |dir| {
        if found.is_some() || invalid {
            return;
        }
        for file in files {
            match fs::read_to_string(dir.join(file)) {
                Ok(value) if value.trim().is_empty() => {}
                Ok(value) => match parse_cpuset_capacity(&value) {
                    Some(capacity) => {
                        found = Some(capacity);
                        return;
                    }
                    None => {
                        invalid = true;
                        return;
                    }
                },
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => {
                    invalid = true;
                    return;
                }
            }
        }
    });
    (!invalid).then_some(found).flatten()
}

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}
