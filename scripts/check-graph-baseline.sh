#!/usr/bin/env bash
set -euo pipefail

# GRAPH-001 keeps the current graph scope as a read-only derived projection.
# Rendering and indexing are future work; this script guards against graph code
# quietly becoming a ledger/workspace authority path.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRAPH_DIR="$ROOT_DIR/crates/core/src/graph"
GRAPH="$GRAPH_DIR/mod.rs"
GRAPH_LINKS="$ROOT_DIR/crates/core/src/graph/links.rs"
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
  MSYS2_ARG_CONV_EXCL="$text" rg --fixed-strings --quiet -- "$text" "$file" || fail "missing '$text' in $file"
}

absent() {
  local file="$1"
  local text="$2"
  if MSYS2_ARG_CONV_EXCL="$text" rg --fixed-strings --quiet -- "$text" "$file"; then
    fail "unexpected '$text' in $file"
  fi
}

contains "$GRAPH" "pub struct GraphDocument"
contains "$GRAPH" "pub struct GraphProjection"
contains "$GRAPH" "pub fn project_documents"
contains "$GRAPH_LINKS" "GraphLinkKind::Wiki"
contains "$GRAPH_LINKS" "GraphLinkKind::Markdown"
contains "$ROOT_DIR/crates/core/src/lib.rs" "pub mod graph;"
contains "$ROOT_DIR/docs/plan/17_tech_stack.md" "Core graph projection"
contains "$ROOT_DIR/docs/plan/17_tech_stack.md" "CLI/HTTP adapter"
contains "$ROOT_DIR/docs/plan/14_commands.md" "deve graph"
contains "$ROOT_DIR/apps/cli/src/main.rs" "Graph {"
contains "$ROOT_DIR/apps/cli/src/dispatch.rs" "commands::graph::run"
contains "$ROOT_DIR/apps/cli/src/commands/graph.rs" "project_repo_graph"
contains "$CLI_GRAPH" "project_documents"
contains "$CLI_GRAPH" "diagnose_projection_local_repo"
contains "$CLI_GRAPH" "GraphProjectionError::DegradedProjectionRequired"
contains "$CLI_GRAPH" "is_degraded_projection_required"
contains "$ROOT_DIR/crates/core/src/protocol/error.rs" "GRAPH_DEGRADED_PROJECTION_REQUIRED"
contains "$ROOT_DIR/apps/cli/src/server/handlers/repo/http.rs" "ServerErrorCode::GraphDegradedProjectionRequired"
contains "$ROOT_DIR/apps/web/src/i18n/server_error.rs" "ServerErrorCode::GraphDegradedProjectionRequired"
contains "$ROOT_DIR/apps/cli/src/server/router.rs" "/api/repo/graph"
contains "$CLI_GRAPH" "--allow-degraded-projection"
contains "$WEB_GRAPH_API" "DegradedProjectionRequired"
contains "$WEB_GRAPH_API" "ServerErrorCode::GraphDegradedProjectionRequired"
contains "$WEB_GRAPH_API" "graph_projection_error_from_server_error"
contains "$WEB_GRAPH_API" "encode_query_component(repo_id)"
contains "$WEB_GRAPH_API" "graph_projection_url_encodes_repo_id_query_component"
contains "$WEB_GRAPH_API" "--allow-degraded-projection"
contains "$WEB_GRAPH_PANEL" "GraphProjectionFetchState::LocalOnly"
contains "$WEB_GRAPH_PANEL" "GraphProjectionFetchState::Degraded"
contains "$WEB_GRAPH_PANEL" "data-deve-graph-state"
contains "$WEB_GRAPH_PANEL" "data-deve-graph-panel=\"readonly\""
contains "$WEB_GRAPH_PANEL" "data-deve-graph-projection-mode=\"readonly-summary\""
contains "$WEB_GRAPH_PANEL" "data-deve-graph-renderer-gate=\"closed\""
contains "$WEB_GRAPH_PANEL" "data-deve-graph-stat-value"
contains "$WEB_GRAPH_PANEL" "\"local-only\""
contains "$WEB_GRAPH_PANEL" "\"blocked\""
contains "$WEB_GRAPH_PANEL" "\"degraded\""
contains "$WEB_GRAPH_PANEL" "\"error\""
contains "$WEB_GRAPH_PANEL" "\"empty\""
contains "$WEB_GRAPH_I18N" "graph_projection_blocked"
contains "$WEB_GRAPH_I18N" "graph_projection_degraded"
contains "$ROOT_DIR/docs/report/graph-baseline-2026-05-01.md" "Graph 当前停靠在只读 projection data surface"

absent "$GRAPH_DIR" "crate::ledger"
absent "$GRAPH_DIR" "std::fs"
absent "$GRAPH_DIR" "redb"
absent "$GRAPH_DIR" "source_control"
absent "$GRAPH_DIR" "search::"

echo "graph-baseline-check: ok"
