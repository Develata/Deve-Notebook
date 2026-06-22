//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "architecture-registry-check";
const DIFF_FILE: &str = "docs/overview/architecture-diff.md";
const DRIFT_MAP: &str = "docs/overview/graph/drift-map.tsv";
const GRAPH_FRAG_DIR: &str = "docs/overview/graph/fragments";
const DOC_LISP: &str = "docs/overview/architecture-doc.lisp";
const CODE_LISP: &str = "docs/overview/architecture-code.lisp";
const OPS_DIR: &str = "docs/features/operations";
const OP_COVERAGE: &str = "docs/features/operation-coverage.md";
const ACCEPTANCE_DIR: &str = "docs/acceptance-cases";

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    let root = ctx.root();
    require_file(root, DIFF_FILE)?;
    require_file(root, DRIFT_MAP)?;
    require_dir(root, GRAPH_FRAG_DIR)?;
    require_file(root, DOC_LISP)?;
    require_file(root, CODE_LISP)?;
    require_dir(root, OPS_DIR)?;
    require_file(root, OP_COVERAGE)?;
    require_dir(root, ACCEPTANCE_DIR)?;

    let diff = read_rel(root, DIFF_FILE)?;
    let drift_map = read_rel(root, DRIFT_MAP)?;
    let graph_fragments = read_tree(root, GRAPH_FRAG_DIR)?;
    let doc_lisp = read_rel(root, DOC_LISP)?;
    let code_lisp = read_rel(root, CODE_LISP)?;
    let op_coverage = read_rel(root, OP_COVERAGE)?;
    let case_set = collect_case_ids(&root.join(ACCEPTANCE_DIR))?;
    if case_set.is_empty() {
        return fail("no acceptance case ids found");
    }

    let flows = extract_registry(
        &diff,
        "<!-- flow-registry:start -->",
        "<!-- flow-registry:end -->",
    )?;
    if flows.is_empty() {
        return fail("flow registry is empty");
    }

    let declared_count = marker_count(&diff, "Flow count")?;
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
            &doc_lisp,
            format!(":label \"{flow}\""),
            "flow missing in doc lisp",
            flow,
        )?;
        require_contains(
            &code_lisp,
            format!(":label \"{flow}\""),
            "flow missing in code lisp",
            flow,
        )?;
    }

    let drift_rows = parse_drift_map(&drift_map)?;
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
        &diff,
        "<!-- drift-registry:start -->",
        "<!-- drift-registry:end -->",
    )?;
    if drifts.is_empty() {
        return fail("drift registry is empty");
    }
    if drifts.len() == 1 && drifts[0] == "none" {
        drifts.clear();
    }

    let active_count = marker_count(&diff, "Active drift count")?;
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

    check_operation_files(root, &doc_lisp, &code_lisp, &op_coverage, &case_set)?;
    println!(
        "{LABEL}: ok ({} flows, {} active drift)",
        flows.len(),
        drifts.len()
    );
    Ok(())
}

fn check_operation_files(
    root: &Path,
    doc_lisp: &str,
    code_lisp: &str,
    op_coverage: &str,
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

fn collect_case_ids(acceptance_dir: &Path) -> Result<BTreeSet<String>> {
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

fn extract_registry(content: &str, start: &str, end: &str) -> Result<Vec<String>> {
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

fn metadata_backtick_value(content: &str, key: &str) -> Option<String> {
    let prefix = format!("- `{key}`:");
    content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .and_then(|line| {
            let mut parts = line.split('`');
            parts.nth(3).map(ToString::to_string)
        })
}

fn metadata_line_value(content: &str, key: &str) -> Option<String> {
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

fn extract_case_refs(content: &str) -> BTreeSet<String> {
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

fn read_rel(root: &Path, rel: &str) -> Result<String> {
    fs::read_to_string(root.join(rel)).with_context(|| format!("{LABEL}: failed to read {rel}"))
}

fn read_tree(root: &Path, rel: &str) -> Result<String> {
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

fn require_file(root: &Path, rel: &str) -> Result<()> {
    if root.join(rel).is_file() {
        Ok(())
    } else {
        fail(format!("missing {}", root.join(rel).display()))
    }
}

fn require_dir(root: &Path, rel: &str) -> Result<()> {
    if root.join(rel).is_dir() {
        Ok(())
    } else {
        fail(format!("missing {}", root.join(rel).display()))
    }
}

fn require_contains(
    haystack: &str,
    needle: impl AsRef<str>,
    message: &str,
    subject: &str,
) -> Result<()> {
    if !haystack.contains(needle.as_ref()) {
        fail(format!("{message}: {subject}"))
    } else {
        Ok(())
    }
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

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn fail<T>(message: impl AsRef<str>) -> Result<T> {
    bail!("{LABEL}: {}", message.as_ref())
}

#[cfg(test)]
#[path = "architecture_registry_tests.rs"]
mod tests;
