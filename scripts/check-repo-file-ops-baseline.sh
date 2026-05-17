#!/usr/bin/env bash
set -euo pipefail

# Repo file operations closure baseline.
# Covers SearchBox file-op shell, document structure WS gate, and server docs handlers.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_filter() {
  local package="$1"
  local filter="$2"
  local output
  local total

  echo "repo-file-ops-baseline: run: cargo test -p $package $filter -- --nocapture"
  if ! output="$(cd "$ROOT_DIR" && cargo test -p "$package" "$filter" -- --nocapture 2>&1)"; then
    printf '%s\n' "$output"
    return 1
  fi
  printf '%s\n' "$output"

  total="$(awk '/^running [0-9]+ tests?/{sum += $2} END {print sum + 0}' <<<"$output")"
  if [[ "$total" -lt 1 ]]; then
    echo "repo-file-ops-baseline: expected at least one executed test for filter '$filter'" >&2
    return 1
  fi
}

run_filter deve_web file_ops
run_filter deve_web file_provider
run_filter deve_cli docs_scope_nonce_gate
run_filter deve_cli docs_create_test
run_filter deve_cli docs_copy_contract
run_filter deve_cli docs_dir_copy
run_filter deve_cli docs_projection_repair
run_filter deve_cli "server::handlers::docs::create::tests"
run_filter deve_cli "server::handlers::docs::delete::tests"
run_filter deve_cli copy_rejects_traversal_source_before_resolving_target
run_filter deve_cli degraded_local
run_filter deve_core source_control_write_gate

echo "repo-file-ops-baseline: ok"
