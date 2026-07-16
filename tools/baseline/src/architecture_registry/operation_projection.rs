//! plan_ref:
//!   - 20_operations_catalog#opid-catalog
//!   - 20_operations_catalog#extension-point-index
//!   - 20_operations_catalog#replacement-point-index
//!   - 20_operations_catalog#configuration-entry-index

use super::{LABEL, OPS_DIR, display_path, fail, require_contains};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn validate_catalog_projection(
    root: &Path,
    plan_operations: &str,
    op_coverage: &str,
    plan_agents: &str,
) -> Result<BTreeMap<String, String>> {
    check_operation_catalog_agent_status(plan_agents)?;
    let plan_flow_ids = plan_operation_flow_ids(plan_operations)?;
    let coverage_flow_ids = coverage_flow_ids(op_coverage)?;
    require_same_flow_ids(&plan_flow_ids, &coverage_flow_ids)?;
    let coverage_ops = coverage_operation_rows(op_coverage)?;
    check_coverage_operation_files(root, &coverage_ops)?;
    Ok(coverage_ops)
}

pub(super) fn check_operation_files(
    root: &Path,
    doc_lisp: &str,
    code_lisp: &str,
    op_coverage: &str,
    coverage_ops: &BTreeMap<String, String>,
    case_set: &BTreeSet<String>,
) -> Result<()> {
    for op_file in operation_files(&root.join(OPS_DIR))? {
        let base = file_name(&op_file)?;
        if base == "00_schema.md" {
            continue;
        }
        let content = fs::read_to_string(&op_file)
            .with_context(|| format!("{LABEL}: failed to read {}", display_path(&op_file)))?;
        let op_ref = format!("operations/{base}");
        require_contains(
            doc_lisp,
            &op_ref,
            "operation file not referenced by doc lisp",
            &base,
        )?;
        require_contains(
            &content,
            "`Related Acceptance Cases`:",
            "operation file missing acceptance refs",
            &base,
        )?;

        let Some(flow_id) = metadata_backtick_value(&content, "Flow ID") else {
            return fail(format!("operation file missing Flow ID: {base}"));
        };
        let Some(coverage_ref) = coverage_ops.get(&flow_id) else {
            return fail(format!("coverage registry missing flow: {flow_id}"));
        };
        if coverage_ref != &op_ref {
            return fail(format!(
                "coverage registry maps {flow_id} to {coverage_ref}, expected {op_ref}"
            ));
        }
        require_contains(
            op_coverage,
            format!("| `{flow_id}` |"),
            "coverage registry missing flow",
            &flow_id,
        )?;
        require_contains(
            op_coverage,
            &op_ref,
            "coverage registry missing file",
            &base,
        )?;

        let Some(acceptance_line) = metadata_line_value(&content, "`Related Acceptance Cases`")
        else {
            return fail(format!("operation file has no acceptance case IDs: {base}"));
        };
        let op_cases = extract_case_refs(&acceptance_line);
        if op_cases.is_empty() {
            return fail(format!("operation file has no acceptance case IDs: {base}"));
        }

        let Some(coverage_row) = coverage_row(op_coverage, &flow_id) else {
            return fail(format!(
                "coverage row has no acceptance case IDs: {flow_id}"
            ));
        };
        let coverage_cases = extract_case_refs(coverage_row);
        if coverage_cases.is_empty() {
            return fail(format!(
                "coverage row has no acceptance case IDs: {flow_id}"
            ));
        }
        if op_cases != coverage_cases {
            return fail(format!("coverage refs differ for: {base}"));
        }
        for case_id in &op_cases {
            if !case_set.contains(case_id) {
                return fail(format!("acceptance case missing: {case_id} in {base}"));
            }
        }

        let op_ids = operation_ids(&content)?;
        if op_ids.is_empty() {
            return fail(format!("operation file has no operation IDs: {base}"));
        }
        for op_id in op_ids {
            require_contains(
                doc_lisp,
                format!(":id {op_id} "),
                "operation ID missing in doc lisp",
                &op_id,
            )?;
            require_contains(
                code_lisp,
                format!(":id {op_id} "),
                "operation ID missing in code lisp",
                &op_id,
            )?;
        }
    }
    Ok(())
}

pub(super) fn plan_operation_flow_ids(content: &str) -> Result<BTreeSet<String>> {
    table_flow_ids(content, "chapter 20 operation catalog")
}

pub(super) fn coverage_flow_ids(content: &str) -> Result<BTreeSet<String>> {
    table_flow_ids(content, "operation coverage registry")
}

fn table_flow_ids(content: &str, source: &str) -> Result<BTreeSet<String>> {
    let re = flow_id_regex()?;
    let mut ids = BTreeSet::new();
    let mut in_flow_table = false;
    for line in content.lines() {
        if !in_flow_table {
            if line.trim_start().starts_with('|') && line.contains("Flow ID") {
                in_flow_table = true;
            }
            continue;
        }
        if !line.trim_start().starts_with('|') {
            break;
        }
        let Some(candidate) = first_table_backtick_value(line) else {
            continue;
        };
        if !re.is_match(&candidate) {
            continue;
        }
        if !ids.insert(candidate.clone()) {
            return fail(format!("duplicate flow id in {source}: {candidate}"));
        }
    }
    if ids.is_empty() {
        return fail(format!("no flow ids found in {source}"));
    }
    Ok(ids)
}

pub(super) fn coverage_operation_rows(content: &str) -> Result<BTreeMap<String, String>> {
    let flow_re = flow_id_regex()?;
    let op_re = Regex::new(r"\]\(\./(operations/[A-Za-z0-9_][A-Za-z0-9_-]*\.md)\)")?;
    let mut rows = BTreeMap::new();
    let mut in_flow_table = false;
    for line in content.lines() {
        if !in_flow_table {
            if line.trim_start().starts_with('|') && line.contains("Flow ID") {
                in_flow_table = true;
            }
            continue;
        }
        if !line.trim_start().starts_with('|') {
            break;
        }
        let Some(flow_id) = first_table_backtick_value(line) else {
            continue;
        };
        if !flow_re.is_match(&flow_id) {
            continue;
        }
        let Some(op_match) = op_re
            .captures(line)
            .and_then(|captures| captures.get(1).map(|match_| match_.as_str().to_string()))
        else {
            return fail(format!(
                "coverage registry missing operation file: {flow_id}"
            ));
        };
        if rows.insert(flow_id.clone(), op_match).is_some() {
            return fail(format!("duplicate coverage registry flow: {flow_id}"));
        }
    }
    if rows.is_empty() {
        return fail("no operation coverage rows found");
    }
    Ok(rows)
}

fn check_coverage_operation_files(
    root: &Path,
    coverage_ops: &BTreeMap<String, String>,
) -> Result<()> {
    let feature_root = root.join("docs/features");
    for (flow_id, op_ref) in coverage_ops {
        let op_path = feature_root.join(op_ref);
        if !op_path.is_file() {
            return fail(format!("coverage operation file missing: {op_ref}"));
        }
        let content = fs::read_to_string(&op_path)
            .with_context(|| format!("{LABEL}: failed to read {}", display_path(&op_path)))?;
        let Some(metadata_flow_id) = metadata_backtick_value(&content, "Flow ID") else {
            return fail(format!("coverage operation file missing Flow ID: {op_ref}"));
        };
        if metadata_flow_id != *flow_id {
            return fail(format!(
                "coverage flow {flow_id} points to {op_ref}, but file declares {metadata_flow_id}"
            ));
        }
    }
    Ok(())
}

fn flow_id_regex() -> Result<Regex> {
    Ok(Regex::new(r"^flow(?:\.[a-z0-9]+(?:-[a-z0-9]+)*)+$")?)
}

fn first_table_backtick_value(line: &str) -> Option<String> {
    if !line.trim_start().starts_with('|') {
        return None;
    }
    line.split('|')
        .nth(1)
        .and_then(|column| column.split('`').nth(1))
        .map(ToString::to_string)
}

pub(super) fn require_same_flow_ids(
    plan: &BTreeSet<String>,
    coverage: &BTreeSet<String>,
) -> Result<()> {
    let missing_in_coverage = set_difference(plan, coverage);
    let extra_in_coverage = set_difference(coverage, plan);
    if missing_in_coverage.is_empty() && extra_in_coverage.is_empty() {
        return Ok(());
    }
    let missing = if missing_in_coverage.is_empty() {
        "none".to_string()
    } else {
        missing_in_coverage.join(", ")
    };
    let extra = if extra_in_coverage.is_empty() {
        "none".to_string()
    } else {
        extra_in_coverage.join(", ")
    };
    fail(format!(
        "chapter 20 / operation coverage flow mismatch; missing in coverage: {missing}; extra in coverage: {extra}"
    ))
}

pub(super) fn check_operation_catalog_agent_status(plan_agents: &str) -> Result<()> {
    const MARKER: &str = "deve_baseline architecture-registry 绑定";
    const SKIP: &str = "no-rust-plan-ref";
    const PLANNED: &str = "planned/no-code-yet";
    const ANCHORS: &[&str] = &[
        "20_operations_catalog#opid-catalog",
        "20_operations_catalog#extension-point-index",
        "20_operations_catalog#replacement-point-index",
        "20_operations_catalog#configuration-entry-index",
    ];

    for anchor in ANCHORS {
        let prefix = format!("| `{anchor}` |");
        let Some(row) = plan_agents.lines().find(|line| line.starts_with(&prefix)) else {
            return fail(format!(
                "docs/plan/AGENTS.md missing registry row: {anchor}"
            ));
        };
        if !row.contains(MARKER) || row.contains(SKIP) || row.contains(PLANNED) {
            return fail(format!(
                "operation catalog registry status must be architecture-registry-bound Rust plan_ref: {anchor}"
            ));
        }
    }

    Ok(())
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

pub(super) fn collect_case_ids(acceptance_dir: &Path) -> Result<BTreeSet<String>> {
    let re = Regex::new(r"case_id:\s+([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)")?;
    let mut ids = BTreeSet::new();
    for file in markdown_files(acceptance_dir)? {
        let content = fs::read_to_string(&file)
            .with_context(|| format!("{LABEL}: failed to read {}", display_path(&file)))?;
        for captures in re.captures_iter(&content) {
            let case_id = captures[1].to_string();
            if !ids.insert(case_id.clone()) {
                return fail(format!("duplicate acceptance case id: {case_id}"));
            }
        }
    }
    Ok(ids)
}

pub(super) fn metadata_backtick_value(content: &str, key: &str) -> Option<String> {
    let prefix = format!("- `{key}`:");
    content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .and_then(|line| {
            let mut parts = line.split('`');
            parts.nth(3).map(ToString::to_string)
        })
}

pub(super) fn metadata_line_value(content: &str, key: &str) -> Option<String> {
    let prefix = format!("- {key}: ");
    content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| line[prefix.len()..].to_string())
}

fn coverage_row<'a>(content: &'a str, flow_id: &str) -> Option<&'a str> {
    let needle = format!("`{flow_id}`");
    content.lines().find(|line| {
        line.split('|')
            .nth(1)
            .is_some_and(|column| column.contains(&needle))
    })
}

pub(super) fn extract_case_refs(content: &str) -> BTreeSet<String> {
    let re = Regex::new(r"[A-Z][A-Z0-9-]*-[0-9]+").expect("valid case ref regex");
    re.find_iter(content)
        .map(|match_| match_.as_str().to_string())
        .collect()
}

fn operation_ids(content: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"^### `([^`]+)`")?;
    Ok(content
        .lines()
        .filter_map(|line| re.captures(line).map(|captures| captures[1].to_string()))
        .collect())
}

fn operation_files(ops_dir: &Path) -> Result<Vec<PathBuf>> {
    markdown_files(ops_dir)
}

fn markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn file_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .with_context(|| {
            format!(
                "{LABEL}: failed to read file name for {}",
                display_path(path)
            )
        })
}
