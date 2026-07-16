//! plan_ref:
//!   - 20_operations_catalog#opid-catalog
//!   - 20_operations_catalog#extension-point-index
//!   - 20_operations_catalog#replacement-point-index
//!   - 20_operations_catalog#configuration-entry-index

mod flow_graph;
mod operation_projection;

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub(super) const LABEL: &str = "architecture-registry-check";
const DIFF_FILE: &str = "docs/overview/architecture-diff.md";
const DRIFT_MAP: &str = "docs/overview/graph/drift-map.tsv";
const GRAPH_FRAG_DIR: &str = "docs/overview/graph/fragments";
const DOC_LISP: &str = "docs/overview/architecture-doc.lisp";
const CODE_LISP: &str = "docs/overview/architecture-code.lisp";
pub(super) const OPS_DIR: &str = "docs/features/operations";
const OP_COVERAGE: &str = "docs/features/operation-coverage.md";
const PLAN_OPERATIONS: &str = "docs/plan/20_operations_catalog.md";
const PLAN_AGENTS: &str = "docs/plan/AGENTS.md";
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
    require_file(root, PLAN_OPERATIONS)?;
    require_file(root, PLAN_AGENTS)?;
    require_dir(root, ACCEPTANCE_DIR)?;

    let diff = read_rel(root, DIFF_FILE)?;
    let drift_map = read_rel(root, DRIFT_MAP)?;
    let graph_fragments = flow_graph::read_tree(root, GRAPH_FRAG_DIR)?;
    let doc_lisp = read_rel(root, DOC_LISP)?;
    let code_lisp = read_rel(root, CODE_LISP)?;
    let op_coverage = read_rel(root, OP_COVERAGE)?;
    let plan_operations = read_rel(root, PLAN_OPERATIONS)?;
    let plan_agents = read_rel(root, PLAN_AGENTS)?;
    let case_set = operation_projection::collect_case_ids(&root.join(ACCEPTANCE_DIR))?;
    if case_set.is_empty() {
        return fail("no acceptance case ids found");
    }

    let coverage_ops = operation_projection::validate_catalog_projection(
        root,
        &plan_operations,
        &op_coverage,
        &plan_agents,
    )?;
    let graph = flow_graph::validate(&diff, &drift_map, &graph_fragments, &doc_lisp, &code_lisp)?;
    operation_projection::check_operation_files(
        root,
        &doc_lisp,
        &code_lisp,
        &op_coverage,
        &coverage_ops,
        &case_set,
    )?;
    println!(
        "{LABEL}: ok ({} flows, {} active drift)",
        graph.flow_count, graph.active_drift_count
    );
    Ok(())
}

fn read_rel(root: &Path, rel: &str) -> Result<String> {
    fs::read_to_string(root.join(rel)).with_context(|| format!("{LABEL}: failed to read {rel}"))
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

pub(super) fn require_contains(
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

pub(super) fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

pub(super) fn fail<T>(message: impl AsRef<str>) -> Result<T> {
    bail!("{LABEL}: {}", message.as_ref())
}

#[cfg(test)]
use flow_graph::extract_registry;
#[cfg(test)]
use operation_projection::{
    check_operation_catalog_agent_status, coverage_flow_ids, coverage_operation_rows,
    extract_case_refs, metadata_backtick_value, metadata_line_value, plan_operation_flow_ids,
    require_same_flow_ids,
};

#[cfg(test)]
#[path = "architecture_registry_tests.rs"]
mod tests;
