#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT_DIR_NATIVE="$ROOT_DIR"
if native_root="$(cd "$ROOT_DIR" && pwd -W 2>/dev/null)"; then
  ROOT_DIR_NATIVE="${native_root//\\//}"
fi
RUN_DESKTOP_NATIVE_PACKAGING_TESTS="${DEVE_NATIVE_PROCESS_ADAPTER_RUN_DESKTOP_NATIVE_PACKAGING_TESTS:-1}"

fail() {
  echo "native-process-adapter-gate-check: $*" >&2
  exit 1
}

is_windows_bash_host() {
  case "$(uname -s 2>/dev/null || printf 'unknown')" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

is_wsl_mounted_workspace() {
  [[ "$ROOT_DIR" == /mnt/* ]] \
    && grep -qi microsoft /proc/version 2>/dev/null
}

select_cargo_bin() {
  if [[ -n "${CARGO_BIN:-}" ]]; then
    command -v "$CARGO_BIN" >/dev/null 2>&1 \
      || fail "configured CARGO_BIN '$CARGO_BIN' was not found"
    return
  fi
  if is_wsl_mounted_workspace && command -v cargo.exe >/dev/null 2>&1; then
    CARGO_BIN="$(command -v cargo.exe)"
  elif command -v cargo >/dev/null 2>&1; then
    CARGO_BIN="$(command -v cargo)"
  else
    fail "cargo command not found"
  fi
}

configure_cargo_target_dir() {
  CARGO_TARGET_ARG="${CARGO_TARGET_DIR:-target/native-process-gate}"
}

run_cargo() {
  local subcommand="$1"
  shift
  run "$CARGO_BIN" "$subcommand" --target-dir "$CARGO_TARGET_ARG" "$@"
}

can_use_rg_for_paths() {
  command -v rg >/dev/null 2>&1 && ! is_windows_bash_host
}

normalize_search_line() {
  local line="$1"
  printf '%s\n' "${line//\\//}"
}

repo_line_matches() {
  local line="$1"
  local rel="$2"
  line="$(normalize_search_line "$line")"
  [[ "$line" == "$ROOT_DIR/$rel":* ]] && return 0
  [[ "$line" == "$ROOT_DIR_NATIVE/$rel":* ]]
}

repo_line_display() {
  local line="$1"
  line="$(normalize_search_line "$line")"
  line="${line#"$ROOT_DIR"/}"
  line="${line#"$ROOT_DIR_NATIVE"/}"
  printf '%s\n' "$line"
}

run() {
  echo "+ $*"
  "$@"
}

contains_fixed() {
  local file="$1"
  local pattern="$2"
  if can_use_rg_for_paths; then
    rg -q --fixed-strings "$pattern" "$file"
  else
    grep -F -- "$pattern" "$file" >/dev/null
  fi
}

search_regex() {
  local pattern="$1"
  shift
  if can_use_rg_for_paths; then
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
      repo_line_matches "$line" "apps/desktop/src/process_runtime.rs" && continue
      repo_line_matches "$line" "apps/desktop/src/process_runtime/launcher.rs" && continue
      fail "native process runtime is only allowed in the Desktop post-gate runtime spike: $(repo_line_display "$line")"
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
  check_contains apps/desktop/src/process_runtime/launcher.rs "env_clear()"
  check_contains apps/desktop/src/process_runtime/validation.rs "executable must be deve_cli"
  check_contains apps/desktop/src/process_runtime/validation.rs "first argv must be serve"
  check_contains apps/desktop/src/process_runtime/validation.rs "argv must be exactly serve --native-loopback --port <port>"
  check_contains apps/desktop/src/process_runtime/validation.rs "argv port must match loopback bind hints"
  check_contains apps/desktop/src/service_entrypoint/mod.rs "DEVE_DESKTOP_LOCAL_SERVICE"
  check_contains apps/desktop/src/service_entrypoint/mod.rs "DEVE_NATIVE_AUTHORITY"
  check_contains apps/desktop/src/service_entrypoint_test.rs "desktop_local_service_entrypoint_env_requires_native_authority_and_local_service"
  check_contains apps/desktop/src/service_entrypoint_test.rs "desktop_local_service_entrypoint_env_rejects_invalid_opt_in_value"
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "\"--native-loopback\""
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "NATIVE_SESSION_BOOTSTRAP_SECRET_ENV"
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "generate_native_session_bootstrap_secret"
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "\"AUTH_SECRET\""
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "\"AUTH_PASS\""
  check_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "password::hash_password"
  check_contains apps/desktop/src/service_entrypoint/mod.rs "health_probe_required_before_bootstrap: true"
  check_contains apps/desktop/src/service_entrypoint/mod.rs "session_handoff_required_before_bootstrap: true"
  check_contains apps/desktop/src/service_bootstrap.rs "run_desktop_local_service_bootstrap"
  check_contains apps/desktop/src/service_bootstrap.rs "DesktopLoopbackHttpProbe"
  check_contains apps/desktop/src/service_bootstrap/probe.rs "/api/node/role"
  check_contains apps/desktop/src/service_bootstrap/probe.rs "/api/auth/status"
  check_contains apps/desktop/src/service_bootstrap/probe.rs "/api/auth/native-session"
  check_contains apps/desktop/src/service_bootstrap/probe.rs "NATIVE_SESSION_BOOTSTRAP_HEADER"
  check_contains apps/desktop/src/service_bootstrap.rs "MissingNativeSessionBootstrapSecret"
  check_contains apps/desktop/src/service_bootstrap.rs "bootstrap_for_web"
  check_contains apps/desktop/src/service_bootstrap.rs "SessionHandoffFailed"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "desktop_tauri_bootstrap_plugin"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "js_init_script"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "on_webview_ready"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "set_cookie"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "NativeSessionCookieRequired"
  check_contains apps/desktop/src/tauri_bootstrap/mod.rs "desktop native session cookie install failed before bootstrap"
  check_contains apps/desktop/src/tauri_entry/mod.rs "desktop_tauri_local_service_bootstrap_from_env"
  check_contains apps/desktop/src/tauri_entry/mod.rs "desktop_tauri_native_session_smoke"
  check_contains apps/desktop/src/tauri_entry/smoke.rs "desktop-native-session-smoke: ok"
  check_contains apps/desktop/src/tauri_entry/mod.rs "DesktopLocalServiceTauriState::new"
  check_contains apps/desktop/src/main.rs "DEVE_DESKTOP_NATIVE_SESSION_SMOKE"
  check_contains apps/cli/src/server/auth/handlers/native_session.rs "NativeSessionBridge"
  check_contains apps/cli/src/server/auth/handlers/native_session.rs "NATIVE_SESSION_BOOTSTRAP_HEADER"
  check_contains apps/cli/src/server/auth/handlers/native_session.rs "issue_once"
  check_contains apps/cli/src/server/router.rs "/api/auth/native-session"
  check_contains apps/cli/src/server/start.rs "runtime::init_auth_runtime"
  check_contains apps/cli/src/server/runtime/auth_runtime.rs "NativeSessionBridge::from_env"
  check_not_contains apps/desktop/src/tauri_entry/mod.rs "start_desktop_local_service_if_enabled"
  check_not_contains apps/desktop/src/tauri_entry/mod.rs "app.manage(Mutex::new(runtime))"
  check_not_contains apps/desktop/src/service_entrypoint/mod.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"
  check_not_contains apps/desktop/src/service_entrypoint/spawn_spec.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"
  check_not_contains apps/desktop/src/service_bootstrap.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"
  check_not_contains apps/desktop/src/service_bootstrap/probe.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"
  check_not_contains apps/desktop/src/tauri_bootstrap/mod.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"
  check_not_contains apps/cli/src/server/auth/handlers/native_session.rs "AUTH_ALLOW_ANONYMOUS_LOCALHOST"

  if [[ -f "$ROOT_DIR/apps/mobile/src/process_runtime.rs" ]] \
    || search_regex 'mod[[:space:]]+process_runtime[[:space:]]*;' "$ROOT_DIR/apps/mobile/src/lib.rs" >/dev/null; then
    fail "mobile process runtime must remain closed"
  fi
}

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"
select_cargo_bin
configure_cargo_target_dir

check_contains crates/core/src/native_adapter/process.rs "DeferredUntilPackagingGate"
check_contains crates/core/src/native_adapter/process.rs "ExplicitNativeAuthorityOptIn"
check_contains crates/core/src/native_adapter/process.rs "child_process_runtime_enabled: false"
check_contains crates/core/src/native_adapter/process.rs "embedded_service_runtime_enabled: false"
check_contains crates/core/src/native_adapter/process.rs "packaging_gate_required: true"
check_contains crates/core/src/native_adapter/process.rs "authority_writes_allowed: false"
check_contains crates/core/src/native_adapter/process.rs "desktop_native_authority_policy_from_env"
check_contains crates/core/src/native_adapter/process.rs "mobile_native_authority_policy_from_env"
check_contains crates/core/src/native_adapter/process.rs "DEVE_NATIVE_AUTHORITY"
check_contains crates/core/src/native_adapter/process.rs "DEVE_DESKTOP_LOCAL_SERVICE"
check_contains crates/core/src/native_adapter/process.rs "DEVE_MOBILE_EMBEDDED_SERVICE"
check_contains crates/core/src/native_adapter/process.rs "ChildProcessRuntimeDisabled"
check_contains crates/core/src/native_adapter/process.rs "bind_existing_endpoint"
check_contains crates/core/src/native_adapter/process.rs "bind_session"
check_contains crates/core/src/native_adapter/process.rs "record_probe_timeout"
check_contains crates/core/src/native_adapter/process.rs "record_process_stopped"
check_contains docs/plan/11_ui_design/index.md "native authority 与本地 service 默认关闭"
check_contains docs/plan/11_ui_design/02_desktop.md "DEVE_NATIVE_AUTHORITY=1"
check_contains docs/plan/11_ui_design/02_desktop.md "DEVE_DESKTOP_LOCAL_SERVICE=1"
check_contains docs/plan/11_ui_design/03_mobile.md "DEVE_MOBILE_EMBEDDED_SERVICE=1"
check_contains docs/features/08_ui_design_02_desktop.md "Native Local Service Opt-in"
check_contains docs/features/08_ui_design_03_mobile.md "Native Embedded Service Opt-in"
check_contains docs/dev-runbook.md "Native Authority Opt-in"
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

run_cargo test --locked -p deve_core --lib native_adapter::process_test -- --nocapture
run_cargo test --locked -p deve_desktop desktop_default_build_defers_real_process_adapter -- --nocapture
run_cargo test --locked -p deve_mobile mobile_default_build_defers_real_process_adapter -- --nocapture
run_cargo test --locked -p deve_cli native_session -- --nocapture
case "$RUN_DESKTOP_NATIVE_PACKAGING_TESTS" in
  1|true|TRUE|yes|YES)
    run_cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture
    run_cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture
    ;;
  0|false|FALSE|no|NO)
    echo "native-process-adapter-gate-check: skip Desktop native-packaging tests for scoped target-host run"
    ;;
  *)
    fail "invalid DEVE_NATIVE_PROCESS_ADAPTER_RUN_DESKTOP_NATIVE_PACKAGING_TESTS: $RUN_DESKTOP_NATIVE_PACKAGING_TESTS"
    ;;
esac
run_cargo test --locked -p deve_desktop process_observation -- --nocapture
run_cargo test --locked -p deve_mobile process_observation -- --nocapture

echo "native-process-adapter-gate-check: ok"
