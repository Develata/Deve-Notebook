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

contains_fixed() {
  local file="$1"
  local pattern="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -q --fixed-strings "$pattern" "$file"
  else
    grep -F -- "$pattern" "$file" >/dev/null
  fi
}

search_regex() {
  local pattern="$1"
  shift
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$@"
  else
    grep -REn -- "$pattern" "$@"
  fi
}

check_contains() {
  local file="$1"
  local pattern="$2"
  contains_fixed "$ROOT_DIR/$file" "$pattern" \
    || fail "missing '$pattern' in $file"
}

check_not_contains() {
  local file="$1"
  local pattern="$2"
  ! contains_fixed "$ROOT_DIR/$file" "$pattern" \
    || fail "unexpected '$pattern' in $file"
}

check_no_process_runtime_leak() {
  local imports
  imports="$(search_regex '(^|[^[:alnum:]_])(std::process|Command::new|tokio::process|\.spawn\()' \
    "$ROOT_DIR/apps/desktop/src" \
    "$ROOT_DIR/apps/mobile/src" \
    "$ROOT_DIR/crates/core/src/native_adapter" || true)"
  if [[ -n "$imports" ]]; then
    while IFS= read -r line; do
      case "$line" in
        "$ROOT_DIR/apps/desktop/src/process_runtime.rs":*) ;;
        *) fail "native process runtime is only allowed in the Desktop post-gate runtime spike: ${line#"$ROOT_DIR"/}" ;;
      esac
    done <<< "$imports"
  fi

  check_contains apps/desktop/src/lib.rs "#[cfg(feature = \"native-packaging\")]"
  check_contains apps/desktop/src/lib.rs "mod process_runtime;"
  check_contains apps/desktop/src/lib.rs "mod service_bootstrap;"
  check_contains apps/desktop/src/lib.rs "mod service_entrypoint;"
  check_contains apps/desktop/src/lib.rs "mod tauri_bootstrap;"
  check_contains apps/desktop/src/process_runtime.rs "DesktopLocalServiceRuntime"
  check_contains apps/desktop/src/process_runtime.rs "DesktopCommandProcessLauncher"
  check_contains apps/desktop/src/process_runtime.rs "validate_desktop_service_command"
  check_contains apps/desktop/src/process_runtime.rs "stop_service"
  check_contains apps/desktop/src/process_runtime.rs "env_clear()"
  check_contains apps/desktop/src/process_runtime.rs "executable must be deve_cli"
  check_contains apps/desktop/src/process_runtime.rs "first argv must be serve"
  check_contains apps/desktop/src/process_runtime.rs "argv must be exactly serve --native-loopback --port <port>"
  check_contains apps/desktop/src/process_runtime.rs "argv port must match loopback bind hints"
  check_contains apps/desktop/src/service_entrypoint.rs "DEVE_DESKTOP_LOCAL_SERVICE"
  check_contains apps/desktop/src/service_entrypoint.rs "\"--native-loopback\""
  check_contains apps/desktop/src/service_entrypoint.rs "health_probe_required_before_bootstrap: true"
  check_contains apps/desktop/src/service_entrypoint.rs "session_handoff_required_before_bootstrap: true"
  check_contains apps/desktop/src/service_bootstrap.rs "run_desktop_local_service_bootstrap"
  check_contains apps/desktop/src/service_bootstrap.rs "DesktopLoopbackHttpProbe"
  check_contains apps/desktop/src/service_bootstrap.rs "/api/node/role"
  check_contains apps/desktop/src/service_bootstrap.rs "/api/auth/status"
  check_contains apps/desktop/src/service_bootstrap.rs "bootstrap_for_web"
  check_contains apps/desktop/src/service_bootstrap.rs "SessionHandoffFailed"
  check_contains apps/desktop/src/tauri_bootstrap.rs "desktop_tauri_bootstrap_plugin"
  check_contains apps/desktop/src/tauri_bootstrap.rs "js_init_script"
  check_contains apps/desktop/src/tauri_entry.rs "desktop_tauri_local_service_bootstrap_from_env"
  check_contains apps/desktop/src/tauri_entry.rs "DesktopLocalServiceTauriState::new"
  check_not_contains apps/desktop/src/tauri_entry.rs "start_desktop_local_service_if_enabled"
  check_not_contains apps/desktop/src/tauri_entry.rs "app.manage(Mutex::new(runtime))"

  if [[ -f "$ROOT_DIR/apps/mobile/src/process_runtime.rs" ]] \
    || search_regex 'mod[[:space:]]+process_runtime[[:space:]]*;' "$ROOT_DIR/apps/mobile/src/lib.rs" >/dev/null; then
    fail "mobile process runtime must remain closed"
  fi
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
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "KeepClosedUntilExplicitRuntimeFeature"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "No real child-process runtime was opened."
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "Desktop macOS"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "Desktop Windows"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "Android shell APK package execution is closed"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "iOS simulator shell package build is closed"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "Mobile process runtime must wait for Android/iOS install/startup evidence"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "Command::new"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "tokio::process"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "direct spawn"
check_contains docs/report/process-runtime-gate-decision-after-target-host-closure-2026-05-15.md "must be a separate implementation batch"

check_no_process_runtime_leak

run cargo test --locked -p deve_core native_adapter::process_test -- --nocapture
run cargo test --locked -p deve_desktop desktop_default_build_defers_real_process_adapter -- --nocapture
run cargo test --locked -p deve_mobile mobile_default_build_defers_real_process_adapter -- --nocapture
run cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture
run cargo test --locked -p deve_desktop process_observation -- --nocapture
run cargo test --locked -p deve_mobile process_observation -- --nocapture

echo "native-process-adapter-gate-check: ok"
