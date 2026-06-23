#!/usr/bin/env bash
set -euo pipefail

# REL-008 runtime recovery smoke.
# Verifies degraded-local write gates, stale sync-scope cleanup, and Web
# reconnect/read-only status contracts using existing targeted tests.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# shellcheck source=scripts/baseline-wrapper.sh
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_filter() {
  local package="$1"
  local filter="$2"
  shift 2
  local expected=("$@")
  local output
  local total

  echo "runtime-recovery-smoke: run: cargo test -p $package $filter -- --nocapture"
  if ! output="$(run_baseline_cargo "$ROOT_DIR" runtime-recovery-smoke test -p "$package" "$filter" -- --nocapture 2>&1)"; then
    printf '%s\n' "$output"
    return 1
  fi
  printf '%s\n' "$output"

  total="$(awk '/^running [0-9]+ tests?/{sum += $2} END {print sum + 0}' <<<"$output")"
  if [[ "$total" -eq 0 ]]; then
    echo "runtime-recovery-smoke: expected at least one test for filter '$filter'" >&2
    return 1
  fi

  local name
  for name in "${expected[@]}"; do
    if ! grep -Eq "^test .*$name .*\\.\\.\\. ok$" <<<"$output"; then
      echo "runtime-recovery-smoke: expected passing test '$name' for filter '$filter'" >&2
      return 1
    fi
  done
}

run_filter \
  deve_cli \
  degraded_local \
  create_doc_rejects_degraded_local_projection_before_mutation \
  degraded_local_source_control_writes_are_rejected_before_mutation \
  browser_writer_registration_rejects_degraded_local_projection

run_filter \
  deve_cli \
  sync_scope_cleanup \
  browser_writer_registration_rejects_stale_scope_nonce_with_scoped_error \
  sync_request_on_unbound_remote_clears_stale_db_and_sync_binding

run_filter \
  deve_web \
  write_gate \
  repo_write_gate_blocks_native_recovery_states \
  repo_write_gate_blocks_remote_branches_as_read_only

run_filter \
  deve_web \
  message_refresh \
  rejects_refresh_after_repo_scope_changes \
  refresh_read_gate_blocks_native_recovery_state

run_filter \
  deve_web \
  status_summary \
  reports_native_service_offline_as_specific_recovery_state \
  reports_session_expired_for_unauthorized_status

run_filter \
  deve_web \
  auth_probe \
  classifies_auth_status_codes_as_invalid \
  keeps_non_auth_failures_unknown

echo "runtime-recovery-smoke: ok"
