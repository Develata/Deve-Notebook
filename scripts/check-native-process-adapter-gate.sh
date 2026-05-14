#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-process-adapter-gate-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_no_process_runtime_leak() {
  local imports
  imports="$(rg -n '(^|[^[:alnum:]_])(std::process|Command::new|tokio::process|\.spawn\()' \
    "$ROOT_DIR/apps/desktop/src" \
    "$ROOT_DIR/apps/mobile/src" \
    "$ROOT_DIR/crates/core/src/native_adapter" || true)"
  [[ -z "$imports" ]] \
    || fail "native process runtime must stay absent until process adapter gate opens: ${imports//$'\n'/; }"
}

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

check_contains crates/core/src/native_adapter/process.rs "DeferredUntilPackagingGate"
check_contains crates/core/src/native_adapter/process.rs "child_process_runtime_enabled: false"
check_contains crates/core/src/native_adapter/process.rs "packaging_gate_required: true"
check_contains crates/core/src/native_adapter/process.rs "authority_writes_allowed: false"
check_contains crates/core/src/native_adapter/process.rs "ChildProcessRuntimeDisabled"
check_contains crates/core/src/native_adapter/process.rs "bind_existing_endpoint"
check_contains crates/core/src/native_adapter/process.rs "bind_session"
check_contains crates/core/src/native_adapter/process.rs "record_probe_timeout"
check_contains crates/core/src/native_adapter/process.rs "record_process_stopped"
check_contains crates/core/src/native_adapter/process_runtime.rs "NativeProcessSpawnSpec"
check_contains crates/core/src/native_adapter/process_runtime.rs "NativeProcessRuntimeSnapshot"
check_contains crates/core/src/native_adapter/process_runtime.rs "NativeProcessRuntimeError"
check_contains crates/core/src/native_adapter/process_runtime.rs "validate_contract"
check_contains apps/desktop/src/shell_test/policy.rs "desktop_default_build_defers_real_process_adapter"
check_contains apps/mobile/src/shell_test/policy.rs "mobile_default_build_defers_real_process_adapter"
check_contains docs/report/desktop-process-runtime-gate-decision-2026-05-14.md 'Decision: `KeepClosedUntilTargetHostPackages`'
check_contains docs/report/desktop-process-runtime-gate-decision-2026-05-14.md "No real child-process runtime was opened."
check_contains docs/report/desktop-process-runtime-gate-decision-2026-05-14.md "Command::new"
check_contains docs/report/desktop-process-runtime-gate-decision-2026-05-14.md "DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1"
check_contains docs/report/desktop-process-runtime-gate-decision-2026-05-14.md "Android/iOS Mobile package execution remains target-host work"

check_no_process_runtime_leak

run cargo test --locked -p deve_core native_adapter::process_test -- --nocapture
run cargo test --locked -p deve_desktop desktop_default_build_defers_real_process_adapter -- --nocapture
run cargo test --locked -p deve_mobile mobile_default_build_defers_real_process_adapter -- --nocapture
run cargo test --locked -p deve_desktop process_observation -- --nocapture
run cargo test --locked -p deve_mobile process_observation -- --nocapture

echo "native-process-adapter-gate-check: ok"
