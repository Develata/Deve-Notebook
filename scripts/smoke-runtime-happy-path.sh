#!/usr/bin/env bash
set -euo pipefail

# REL-007 runtime happy path smoke.
# Uses the in-process Axum/WebSocket harness with a temporary repo.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run() {
  echo "runtime-happy-path-smoke: run: $*"
  (cd "$ROOT_DIR" && "$@")
}

run cargo test -p deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope -- --nocapture
run cargo test -p deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready -- --nocapture
run cargo test -p deve_cli ws_edit_after_register_writer_emits_new_op_and_ack -- --nocapture
run cargo test -p deve_cli ws_open_doc_and_history_read_back_registered_edit -- --nocapture
run cargo test -p deve_web restore_runs_only_on_clean_reconnect_edge -- --nocapture

echo "runtime-happy-path-smoke: ok"
