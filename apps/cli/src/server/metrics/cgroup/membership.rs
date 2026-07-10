//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! `/proc` cgroup membership and mountinfo parsing.

use std::path::{Path, PathBuf};

pub(super) struct Hierarchy {
    pub(super) mount: PathBuf,
    pub(super) group: PathBuf,
    pub(super) global_root: bool,
}

pub(super) fn unified_hierarchies(membership: &str, mountinfo: &str) -> Vec<Hierarchy> {
    let Some(member_path) = unified_membership_path(membership) else {
        return Vec::new();
    };
    parse_mounts(mountinfo)
        .into_iter()
        .filter(|mount| mount.fs_type == "cgroup2")
        .filter_map(|mount| mount.resolve(member_path))
        .collect()
}

pub(super) fn controller_hierarchies(
    membership: &str,
    mountinfo: &str,
    controller: &str,
) -> Vec<Hierarchy> {
    let Some(member_path) = legacy_membership_path(membership, controller) else {
        return Vec::new();
    };
    parse_mounts(mountinfo)
        .into_iter()
        .filter(|mount| {
            mount.fs_type == "cgroup"
                && mount
                    .super_options
                    .split(',')
                    .any(|candidate| candidate == controller)
        })
        .filter_map(|mount| mount.resolve(member_path))
        .collect()
}

pub(super) fn walk_ancestors(hierarchy: &Hierarchy, mut visit: impl FnMut(&Path)) {
    if !hierarchy.group.starts_with(&hierarchy.mount) {
        return;
    }
    let mut current = hierarchy.group.as_path();
    loop {
        visit(current);
        if current == hierarchy.mount {
            break;
        }
        let Some(parent) = current
            .parent()
            .filter(|path| path.starts_with(&hierarchy.mount))
        else {
            break;
        };
        current = parent;
    }
}

struct CgroupMount {
    hierarchy_root: String,
    mount_point: PathBuf,
    fs_type: String,
    super_options: String,
}

impl CgroupMount {
    fn resolve(self, membership: &str) -> Option<Hierarchy> {
        let global_root = logical_segments(&self.hierarchy_root)?.is_empty();
        let relative = logical_relative_path(&self.hierarchy_root, membership)?;
        let group = self.mount_point.join(relative);
        Some(Hierarchy {
            mount: self.mount_point,
            group,
            global_root,
        })
    }
}

fn parse_mounts(mountinfo: &str) -> Vec<CgroupMount> {
    mountinfo.lines().filter_map(parse_mount).collect()
}

fn parse_mount(line: &str) -> Option<CgroupMount> {
    let (left, right) = line.split_once(" - ")?;
    let left_fields: Vec<&str> = left.split_whitespace().collect();
    let mut right_fields = right.split_whitespace();
    let fs_type = right_fields.next()?;
    right_fields.next()?;
    let super_options = right_fields.next()?;
    if left_fields.len() < 5 || !matches!(fs_type, "cgroup" | "cgroup2") {
        return None;
    }
    Some(CgroupMount {
        hierarchy_root: decode_mountinfo_field(left_fields[3])?,
        mount_point: PathBuf::from(decode_mountinfo_field(left_fields[4])?),
        fs_type: fs_type.to_string(),
        super_options: super_options.to_string(),
    })
}

fn unified_membership_path(membership: &str) -> Option<&str> {
    membership.lines().find_map(|line| {
        let mut fields = line.splitn(3, ':');
        let hierarchy = fields.next()?;
        let controllers = fields.next()?;
        let path = fields.next()?;
        (hierarchy == "0" && controllers.is_empty()).then_some(path)
    })
}

fn legacy_membership_path<'a>(membership: &'a str, controller: &str) -> Option<&'a str> {
    membership.lines().find_map(|line| {
        let mut fields = line.splitn(3, ':');
        fields.next()?;
        let controllers = fields.next()?;
        let path = fields.next()?;
        controllers
            .split(',')
            .any(|candidate| candidate == controller)
            .then_some(path)
    })
}

fn logical_relative_path(root: &str, membership: &str) -> Option<PathBuf> {
    let root = logical_segments(root)?;
    let membership = logical_segments(membership)?;
    membership
        .starts_with(&root)
        .then(|| membership[root.len()..].iter().collect())
}

fn logical_segments(path: &str) -> Option<Vec<&str>> {
    if !path.starts_with('/') {
        return None;
    }
    let segments: Vec<_> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    segments
        .iter()
        .all(|segment| !matches!(*segment, "." | ".."))
        .then_some(segments)
}

fn decode_mountinfo_field(field: &str) -> Option<String> {
    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() {
            let octal = &field[index + 1..index + 4];
            if octal.bytes().all(|byte| matches!(byte, b'0'..=b'7')) {
                decoded.push(u8::from_str_radix(octal, 8).ok()?);
                index += 4;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).ok()
}
