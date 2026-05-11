#!/usr/bin/env bash
set -euo pipefail

# REL-007 runtime happy path smoke.
# Uses the in-process Axum/WebSocket harness with a temporary repo.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_test() {
  local package="$1"
  local filter="$2"
  local output

  echo "runtime-happy-path-smoke: run: cargo test -p $package $filter -- --nocapture"
  if ! output="$(cd "$ROOT_DIR" && cargo test -p "$package" "$filter" -- --nocapture 2>&1)"; then
    printf '%s\n' "$output"
    return 1
  fi
  printf '%s\n' "$output"
  if ! grep -Fq "running 1 test" <<<"$output"; then
    echo "runtime-happy-path-smoke: expected exactly one test for filter '$filter'" >&2
    return 1
  fi
  if ! grep -Fq "$filter" <<<"$output"; then
    echo "runtime-happy-path-smoke: test output did not mention filter '$filter'" >&2
    return 1
  fi
}

run_test deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope
run_test deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready
run_test deve_cli ws_edit_after_register_writer_emits_new_op_and_ack
run_test deve_cli ws_open_doc_and_history_read_back_registered_edit
run_test deve_web restore_runs_only_on_clean_reconnect_edge

echo "runtime-happy-path-smoke: ok"
