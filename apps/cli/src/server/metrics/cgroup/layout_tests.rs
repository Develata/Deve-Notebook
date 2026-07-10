use super::layout::CgroupSources;
use super::memory_used_mb;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn cgroup_v2_memory_usage_precedes_host_meminfo() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let mount = temp.path().join("cgroup2");
    let group = mount.join("deve.slice");
    fs::create_dir_all(&group)?;
    fs::write(group.join("memory.current"), "4194304\n")?;
    let sources = CgroupSources::from_proc(
        "0::/deve.slice\n",
        &mountinfo_line(29, "/", &mount, "cgroup2", "rw"),
    );
    let host_meminfo = "MemTotal: 8388608 kB\nMemAvailable: 1048576 kB\n";

    assert_eq!(
        memory_used_mb(sources.memory_usage_bytes(), host_meminfo),
        4
    );
    Ok(())
}

#[test]
fn cgroup_mountinfo_maps_unified_mountpoint() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let mount = temp.path().join("unified");
    let group = mount.join("workload");
    fs::create_dir_all(&group)?;
    fs::write(group.join("memory.current"), "5242880\n")?;
    let sources = CgroupSources::from_proc(
        "0::/workload\n",
        &mountinfo_line(30, "/", &mount, "cgroup2", "rw"),
    );

    assert_eq!(sources.memory_usage_bytes(), Some(5_242_880));
    Ok(())
}

#[test]
fn cgroup_parent_quota_limits_leaf_capacity() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let mount = temp.path().join("cgroup2");
    let parent = mount.join("parent");
    let leaf = parent.join("leaf");
    fs::create_dir_all(&leaf)?;
    fs::write(parent.join("cpu.max"), "100000 100000\n")?;
    fs::write(leaf.join("cpu.max"), "max 100000\n")?;
    fs::write(leaf.join("cpu.stat"), "usage_usec 100000\n")?;
    fs::write(leaf.join("cpuset.cpus.effective"), "0-7\n")?;
    let sources = CgroupSources::from_proc(
        "0::/parent/leaf\n",
        &mountinfo_line(31, "/", &mount, "cgroup2", "rw"),
    );

    let source = sources.cpu_usage_source().expect("v2 cpu source");
    assert_eq!(source.capacity_cores(), 1.0);
    assert_eq!(source.read_usage_micros(), Some(100_000));
    Ok(())
}

#[test]
fn cgroup_private_mount_root_requires_quota_file() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let mount = temp.path().join("private-cgroup2");
    fs::create_dir_all(&mount)?;
    fs::write(mount.join("cpu.stat"), "usage_usec 100000\n")?;
    fs::write(mount.join("cpuset.cpus.effective"), "0-3\n")?;
    let mountinfo = mountinfo_line(32, "/private", &mount, "cgroup2", "rw");
    let sources = CgroupSources::from_proc("0::/private\n", &mountinfo);
    assert!(sources.cpu_usage_source().is_none());

    fs::write(mount.join("cpu.max"), "50000 100000\n")?;
    let sources = CgroupSources::from_proc("0::/private\n", &mountinfo);
    assert_eq!(
        sources
            .cpu_usage_source()
            .expect("private v2 cpu source")
            .capacity_cores(),
        0.5
    );
    Ok(())
}

#[test]
fn cgroup_hybrid_falls_back_to_v1_controllers() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let unified = temp.path().join("unified");
    let memory = temp.path().join("memory");
    let cpu = temp.path().join("cpu,cpuacct");
    let cpuset = temp.path().join("cpuset");
    fs::create_dir_all(unified.join("workload"))?;
    fs::create_dir_all(memory.join("legacy"))?;
    fs::create_dir_all(cpu.join("legacy"))?;
    fs::create_dir_all(cpuset.join("legacy"))?;
    fs::write(memory.join("legacy/memory.usage_in_bytes"), "8388608\n")?;
    fs::write(cpu.join("legacy/cpuacct.usage"), "250000000\n")?;
    fs::write(cpu.join("legacy/cpu.cfs_quota_us"), "100000\n")?;
    fs::write(cpu.join("legacy/cpu.cfs_period_us"), "100000\n")?;
    fs::write(cpu.join("cpu.cfs_quota_us"), "-1\n")?;
    fs::write(cpu.join("cpu.cfs_period_us"), "100000\n")?;
    fs::write(cpuset.join("legacy/cpuset.cpus"), "0-3\n")?;
    let membership = "0::/workload\n5:memory:/legacy\n4:cpu,cpuacct:/legacy\n3:cpuset:/legacy\n";
    let mountinfo = [
        mountinfo_line(40, "/", &unified, "cgroup2", "rw"),
        mountinfo_line(41, "/", &memory, "cgroup", "rw,memory"),
        mountinfo_line(42, "/", &cpu, "cgroup", "rw,cpu,cpuacct"),
        mountinfo_line(43, "/", &cpuset, "cgroup", "rw,cpuset"),
    ]
    .concat();
    let sources = CgroupSources::from_proc(membership, &mountinfo);

    assert_eq!(sources.memory_usage_bytes(), Some(8_388_608));
    let source = sources.cpu_usage_source().expect("v1 cpu source");
    assert_eq!(source.capacity_cores(), 1.0);
    assert_eq!(source.read_usage_micros(), Some(250_000));
    Ok(())
}

#[test]
fn cgroup_incomplete_capacity_rejects_source() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let mount = temp.path().join("cgroup2");
    let group = mount.join("broken");
    fs::create_dir_all(&group)?;
    fs::write(mount.join("cpu.max"), "max 100000\n")?;
    fs::write(group.join("cpu.max"), "broken\n")?;
    fs::write(group.join("cpu.stat"), "usage_usec 100000\n")?;
    fs::write(group.join("cpuset.cpus.effective"), "0-3\n")?;
    let sources = CgroupSources::from_proc(
        "0::/broken\n",
        &mountinfo_line(50, "/", &mount, "cgroup2", "rw"),
    );
    assert!(sources.cpu_usage_source().is_none());

    fs::write(group.join("cpu.max"), "max 100000\n")?;
    fs::remove_file(group.join("cpuset.cpus.effective"))?;
    let sources = CgroupSources::from_proc(
        "0::/broken\n",
        &mountinfo_line(51, "/", &mount, "cgroup2", "rw"),
    );
    assert!(sources.cpu_usage_source().is_none());
    Ok(())
}

fn mountinfo_line(
    id: u32,
    hierarchy_root: &str,
    mount_point: &Path,
    fs_type: &str,
    super_options: &str,
) -> String {
    let mount_point = mount_point.to_string_lossy().replace('\\', "/");
    format!("{id} 1 0:{id} {hierarchy_root} {mount_point} rw - {fs_type} cgroup {super_options}\n")
}
