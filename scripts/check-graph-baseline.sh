#!/usr/bin/env bash
set -euo pipefail

# GRAPH-001 keeps the current graph scope as a read-only derived projection.
# Rendering and indexing are future work; this script guards against graph code
# quietly becoming a ledger/workspace authority path.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRAPH="$ROOT_DIR/crates/core/src/graph/mod.rs"
CLI_GRAPH="$ROOT_DIR/apps/cli/src/graph_projection.rs"
WEB_GRAPH_API="$ROOT_DIR/apps/web/src/api/graph.rs"
WEB_GRAPH_PANEL="$ROOT_DIR/apps/web/src/components/sidebar/source_control/graph_panel.rs"
WEB_GRAPH_I18N="$ROOT_DIR/apps/web/src/i18n/source_control_graph.rs"

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
contains "$ROOT_DIR/apps/cli/src/commands/graph.rs" "project_repo_graph"
contains "$CLI_GRAPH" "project_documents"
contains "$CLI_GRAPH" "diagnose_projection_local_repo"
contains "$ROOT_DIR/apps/cli/src/server/router.rs" "/api/repo/graph"
contains "$CLI_GRAPH" "--allow-degraded-projection"
contains "$WEB_GRAPH_API" "DegradedProjectionRequired"
contains "$WEB_GRAPH_API" "--allow-degraded-projection"
contains "$WEB_GRAPH_PANEL" "GraphProjectionFetchState::LocalOnly"
contains "$WEB_GRAPH_PANEL" "GraphProjectionFetchState::Degraded"
contains "$WEB_GRAPH_PANEL" "data-deve-graph-state"
contains "$WEB_GRAPH_PANEL" "\"local-only\""
contains "$WEB_GRAPH_PANEL" "\"blocked\""
contains "$WEB_GRAPH_PANEL" "\"degraded\""
contains "$WEB_GRAPH_PANEL" "\"error\""
contains "$WEB_GRAPH_PANEL" "\"empty\""
contains "$WEB_GRAPH_I18N" "graph_projection_blocked"
contains "$WEB_GRAPH_I18N" "graph_projection_degraded"
contains "$ROOT_DIR/docs/report/next-tasks.md" "P3-13 Graph visualization read-only CLI projection surface 已关闭"

absent "$ROOT_DIR/docs/report/next-tasks.md" "Graph visualization next step | P3-13"

absent "$GRAPH" "crate::ledger"
absent "$GRAPH" "std::fs"
absent "$GRAPH" "redb"
absent "$GRAPH" "source_control"
absent "$GRAPH" "search::"

echo "graph-baseline-check: ok"
