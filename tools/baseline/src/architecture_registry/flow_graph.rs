//! plan_ref:
//!   - 20_operations_catalog#opid-catalog

use super::{LABEL, display_path, fail, require_contains};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct GraphSummary {
    pub(super) flow_count: usize,
    pub(super) active_drift_count: usize,
}

pub(super) fn validate(
    diff: &str,
    drift_map: &str,
    graph_fragments: &str,
    doc_lisp: &str,
    code_lisp: &str,
) -> Result<GraphSummary> {
    let flows = extract_registry(
        diff,
        "<!-- flow-registry:start -->",
        "<!-- flow-registry:end -->",
    )?;
    if flows.is_empty() {
        return fail("flow registry is empty");
    }

    let declared_count = marker_count(diff, "Flow count")?;
    if flows.len() != declared_count {
        return fail(format!(
            "Flow count says {declared_count} but registry has {}",
            flows.len()
        ));
    }

    let mut flow_set = BTreeSet::new();
    for flow in &flows {
        if !flow_set.insert(flow.clone()) {
            return fail(format!("duplicate flow registry entry: {flow}"));
        }
        require_contains(
            doc_lisp,
            format!(":label \"{flow}\""),
            "flow missing in doc lisp",
            flow,
        )?;
        require_contains(
            code_lisp,
            format!(":label \"{flow}\""),
            "flow missing in code lisp",
            flow,
        )?;
    }

    let drift_rows = parse_drift_map(drift_map)?;
    for (flow, root_name) in &drift_rows {
        if !flow_set.contains(flow) {
            return fail(format!("drift map flow not in registry: {flow}"));
        }
        if root_name.is_empty() {
            return fail(format!("drift map root missing for: {flow}"));
        }
        let spine = format!("user_{root_name}_spine");
        if !graph_fragments.contains(&spine) {
            return fail(format!("spine missing for: {flow} -> {root_name}"));
        }
    }

    for flow in &flows {
        if !drift_rows.contains_key(flow) {
            return fail(format!("flow missing from drift map: {flow}"));
        }
    }

    let mut drifts = extract_registry(
        diff,
        "<!-- drift-registry:start -->",
        "<!-- drift-registry:end -->",
    )?;
    if drifts.is_empty() {
        return fail("drift registry is empty");
    }
    if drifts.len() == 1 && drifts[0] == "none" {
        drifts.clear();
    }

    let active_count = marker_count(diff, "Active drift count")?;
    if drifts.len() != active_count {
        return fail(format!(
            "Active drift count says {active_count} but registry has {}",
            drifts.len()
        ));
    }
    for drift in &drifts {
        if !flow_set.contains(drift) {
            return fail(format!("drift not in flow registry: {drift}"));
        }
    }

    Ok(GraphSummary {
        flow_count: flows.len(),
        active_drift_count: drifts.len(),
    })
}

pub(super) fn extract_registry(content: &str, start: &str, end: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"`([^`]+)`")?;
    let mut in_block = false;
    let mut items = Vec::new();
    for line in content.lines() {
        if line == start {
            in_block = true;
            continue;
        }
        if line == end {
            in_block = false;
        }
        if in_block && let Some(captures) = re.captures(line) {
            items.push(captures[1].to_string());
        }
    }
    Ok(items)
}

fn marker_count(content: &str, label: &str) -> Result<usize> {
    let pattern = format!(r"{label}:\s*`([0-9]+)`");
    let re = Regex::new(&pattern)?;
    let Some(captures) = re.captures(content) else {
        return fail(format!("{label} is missing or invalid"));
    };
    captures[1]
        .parse()
        .with_context(|| format!("{LABEL}: failed to parse {label}"))
}

fn parse_drift_map(content: &str) -> Result<BTreeMap<String, String>> {
    let mut rows = BTreeMap::new();
    for line in content.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut columns = line.split('\t');
        let flow = columns.next().unwrap_or_default().to_string();
        let root = columns.next().unwrap_or_default().to_string();
        if flow.is_empty() {
            continue;
        }
        if columns.next().is_some() {
            return fail(format!("drift map row has extra columns: {flow}"));
        }
        rows.insert(flow, root);
    }
    Ok(rows)
}

pub(super) fn read_tree(root: &Path, rel: &str) -> Result<String> {
    let mut content = String::new();
    for file in tree_files(&root.join(rel))? {
        content.push_str(
            &fs::read_to_string(&file)
                .with_context(|| format!("{LABEL}: failed to read {}", display_path(&file)))?,
        );
        content.push('\n');
    }
    Ok(content)
}

fn tree_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_tree_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_tree_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_tree_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}
