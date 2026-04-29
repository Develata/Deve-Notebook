#!/usr/bin/env bash
set -euo pipefail

# GRAPH-001 keeps the current graph scope as a read-only derived projection.
# Rendering and indexing are future work; this script guards against graph code
# quietly becoming a ledger/workspace authority path.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRAPH="$ROOT_DIR/crates/core/src/graph/mod.rs"

fail() {
  echo "graph-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet -- "$text" "$file" || fail "missing '$text' in $file"
}

absent() {
  local file="$1"
  local text="$2"
  if rg --fixed-strings --quiet -- "$text" "$file"; then
    fail "unexpected '$text' in $file"
  fi
}

contains "$GRAPH" "pub struct GraphDocument"
contains "$GRAPH" "pub struct GraphProjection"
contains "$GRAPH" "pub fn project_documents"
contains "$GRAPH" "GraphLinkKind::Wiki"
contains "$GRAPH" "GraphLinkKind::Markdown"
contains "$ROOT_DIR/crates/core/src/lib.rs" "pub mod graph;"
contains "$ROOT_DIR/docs/plan/14_tech_stack.md" "Core read-only projection"
contains "$ROOT_DIR/docs/plan/14_tech_stack.md" "CLI JSON surface"
contains "$ROOT_DIR/docs/plan/12_commands.md" "deve graph"
contains "$ROOT_DIR/apps/cli/src/main.rs" "Graph {"
contains "$ROOT_DIR/apps/cli/src/dispatch.rs" "commands::graph::run"
contains "$ROOT_DIR/apps/cli/src/commands/graph.rs" "project_documents"
contains "$ROOT_DIR/apps/cli/src/commands/graph.rs" "diagnose_projection_local_repo"
contains "$ROOT_DIR/apps/cli/src/commands/graph.rs" "--allow-degraded-projection"
contains "$ROOT_DIR/docs/report/next-tasks.md" "P3-13 Graph visualization read-only CLI projection surface 已关闭"

absent "$ROOT_DIR/docs/report/next-tasks.md" "Graph visualization next step | P3-13"

absent "$GRAPH" "crate::ledger"
absent "$GRAPH" "std::fs"
absent "$GRAPH" "redb"
absent "$GRAPH" "source_control"
absent "$GRAPH" "search::"

echo "graph-baseline-check: ok"
