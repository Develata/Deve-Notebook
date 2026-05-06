#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-track-boundary-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_not_contains() {
  local file="$1"
  local pattern="$2"
  ! rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "unexpected '$pattern' in $file"
}

check_no_packaging_dependency_leak() {
  local cargo_file
  local tauri_manifest_pattern="(^[[:space:]]*[\"']?(tauri|tauri-build)[\"']?[[:space:]]*=|package[[:space:]]*=[[:space:]]*[\"'](tauri|tauri-build)[\"']|^[[:space:]]*\[[^]]*(dependencies|dev-dependencies|build-dependencies)\.[\"']?(tauri|tauri-build)[\"']?[[:space:]]*\])"
  while IFS= read -r cargo_file; do
    if rg -iq "$tauri_manifest_pattern" "$cargo_file"; then
      fail "native packaging dependency is not allowed yet: ${cargo_file#"$ROOT_DIR"/}"
    fi
  done < <(find "$ROOT_DIR" -path "$ROOT_DIR/target" -prune -o -name Cargo.toml -type f -print)

  if rg -n '(^|[^[:alnum:]_])(use[[:space:]]+tauri|tauri::)' \
    "$ROOT_DIR/apps" "$ROOT_DIR/crates" >/dev/null; then
    fail "native packaging runtime imports must stay absent until the packaging gate opens"
  fi
}

check_no_process_runtime_leak() {
  if rg -n '(^|[^[:alnum:]_])(std::process|Command::new|tokio::process|\.spawn\()' \
    "$ROOT_DIR/apps/desktop/src" "$ROOT_DIR/apps/mobile/src" >/dev/null; then
    fail "native process runtime must stay absent from desktop/mobile skeletons until the process adapter gate opens"
  fi
}

check_contains Cargo.toml '"apps/desktop"'
check_contains Cargo.toml '"apps/mobile"'

check_contains apps/desktop/Cargo.toml "native-packaging = []"
check_contains apps/mobile/Cargo.toml "native-packaging = []"
check_contains apps/desktop/src/lib.rs "#[cfg(feature = \"native-packaging\")]"
check_contains apps/desktop/src/packaging.rs "runtime_crate: \"tauri\""
check_contains apps/desktop/src/packaging.rs "build_crate: \"tauri-build\""
check_contains apps/desktop/src/packaging.rs "status: \"planned\""
check_contains apps/desktop/src/packaging.rs "DesktopPackagingAuthority::Ledger"
check_contains apps/desktop/src/packaging.rs "DesktopPackagingCapability::Installer"
check_contains apps/desktop/src/packaging_test.rs "desktop_packaging_scaffold_is_feature_gated_and_planned"
check_contains apps/desktop/src/shell_test.rs "CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY"
check_contains apps/desktop/src/shell_test.rs "CURRENT_NATIVE_PROCESS_ADAPTER_POLICY"
check_contains apps/mobile/src/lib.rs "#[cfg(feature = \"native-packaging\")]"
check_contains apps/mobile/src/packaging.rs "runtime_crate: \"tauri\""
check_contains apps/mobile/src/packaging.rs "build_crate: \"tauri-build\""
check_contains apps/mobile/src/packaging.rs "status: \"planned\""
check_contains apps/mobile/src/packaging.rs "MobilePackagingAuthority::Ledger"
check_contains apps/mobile/src/packaging.rs "MobilePackagingCapability::PermissionBridge"
check_contains apps/mobile/src/packaging.rs "MobilePackagingCapability::StorePackage"
check_contains apps/mobile/src/packaging_test.rs "mobile_packaging_scaffold_is_feature_gated_and_planned"
check_contains apps/mobile/src/shell_test.rs "CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY"
check_contains apps/mobile/src/shell_test.rs "CURRENT_NATIVE_PROCESS_ADAPTER_POLICY"
check_contains crates/core/src/native_adapter/packaging.rs "DeferredUntilRuntimeBatch"
check_contains crates/core/src/native_adapter/packaging.rs "real_tauri_dependencies_allowed: false"
check_contains crates/core/src/native_adapter/process.rs "DeferredUntilPackagingGate"
check_contains crates/core/src/native_adapter/process.rs "child_process_runtime_enabled: false"
check_contains crates/core/src/native_adapter/process.rs "record_probe_timeout"
check_contains crates/core/src/native_adapter/process.rs "record_process_stopped"
check_contains crates/core/src/native_adapter/supervisor.rs "record_process_snapshot"
check_contains crates/core/src/native_adapter/supervisor_test.rs "process_snapshot_drives_health_and_session_handoff"
check_contains crates/core/src/native_adapter/supervisor_test.rs "process_probe_timeout_snapshot_consumes_retry_budget"
check_contains crates/core/src/native_adapter/supervisor_test.rs "process_shutdown_snapshot_enters_restart_path"
check_contains crates/core/src/native_adapter/contract_test.rs "native_reprobe_before_write_requires_full_runtime_readiness"
check_contains apps/web/src/api/service/tests.rs "native_runtime_readiness_requires_node_role_writer_and_current_scope"
check_contains apps/web/src/api/service.rs "native_runtime_readiness_for_untracked"
check_contains apps/web/src/api/connection_role.rs "NODE_ROLE_PROBE_RETRIES"
check_contains apps/web/src/api/connection_role.rs "NODE_ROLE_PROBE_TIMEOUT_MS"
check_contains apps/web/src/api/connection_role.rs "stale_node_role_probe_results_do_not_mutate_current_connection"
check_contains apps/desktop/src/shell.rs "record_process_snapshot"
check_contains apps/desktop/src/shell_recovery_test.rs "desktop_probe_timeout_observation_uses_process_snapshot"
check_contains apps/desktop/src/shell_recovery_test.rs "desktop_process_shutdown_observation_uses_process_snapshot"
check_contains apps/desktop/src/shell_test.rs "missing_node_role"
check_contains apps/mobile/src/shell.rs "record_process_snapshot"
check_contains apps/mobile/src/shell_recovery_test.rs "mobile_probe_timeout_observation_uses_process_snapshot"
check_contains apps/mobile/src/shell_recovery_test.rs "mobile_process_shutdown_observation_uses_process_snapshot"
check_contains apps/mobile/src/shell_test.rs "missing_node_role"
check_contains apps/web/src/hooks/use_core/write_gate_logic.rs "node_role_readable"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "repo_write_gate_requires_node_role_readable"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "repo_source_control_read_gate_reports_node_role_probe_failure_before_read_only"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "repo_source_control_read_gate_requires_node_role_for_local_refresh"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "repo_source_control_read_gate_allows_remote_branch_reads_without_node_role"
check_contains apps/web/src/hooks/use_core/write_gate_tests.rs "repo_write_gate_reports_node_role_probe_failure_before_snapshot_loading"
check_contains apps/web/src/hooks/use_core/status_summary_tests.rs "reports_repo_handshake_until_node_role_is_readable"
check_contains apps/web/src/hooks/use_core/status_summary_tests.rs "reports_native_reprobe_when_node_role_probe_failed"
check_contains apps/web/src/hooks/use_core/status_summary_tests.rs "reports_native_reprobe_before_snapshot_loading_when_node_role_probe_failed"
check_contains apps/web/src/editor/delta_input_gate.rs "blocks_delta_when_runtime_write_gate_blocks"
check_contains apps/web/src/editor/mod.rs "editor_read_only_gate_blocks_native_runtime_write_gate"
check_contains apps/web/src/editor/hook_playback.rs "playback_read_only_gate_blocks_native_runtime_write_gate"
check_contains apps/web/src/editor/delta_input.rs "repo_write_block_untracked"
check_contains apps/web/src/editor/sync/history_resend.rs "repo_write_block_untracked"
check_contains apps/web/src/editor/sync/dispatch_payload.rs "write_ready_resend_blocks_when_native_runtime_readiness_fails"
check_contains apps/web/src/components/chat/actions_apply.rs "repo_write_block_untracked"
check_no_packaging_dependency_leak
check_no_process_runtime_leak

check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-current-native-boundary}"
check_contains docs/plan/08_ui_design_02_desktop.md "**Current Native Boundary**"
check_contains docs/plan/08_ui_design_02_desktop.md "**Post-Gate Target**"
check_contains docs/plan/08_ui_design_02_desktop.md "post-gate normative target"
check_contains docs/plan/08_ui_design_02_desktop.md "Tauri v2 native packaging"
check_contains docs/plan/08_ui_design_02_desktop.md "native-packaging"
check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-packaging-scaffold}"
check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-packaging-dependency-gate-decision}"
check_contains docs/plan/08_ui_design_02_desktop.md "Native adapter **MUST NOT**"
check_contains docs/plan/08_ui_design_02_desktop.md "service readiness/offline"
check_contains docs/plan/08_ui_design_02_desktop.md "loopback/IPC endpoint"
check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-native-adapter-contract}"
check_contains docs/plan/08_ui_design_02_desktop.md "NativeEndpointReady { http_base, ws_base, node_role, session_bound }"
check_contains docs/plan/08_ui_design_02_desktop.md "writer_ready(repo_id, scope_nonce)"
check_contains docs/plan/08_ui_design_02_desktop.md "09_auth.md#unauthorized-disconnected-ui"
check_contains docs/plan/08_ui_design_02_desktop.md "native 层不得直接写 ledger/vault/source-control/search index"

check_contains docs/plan/08_ui_design_03_mobile.md "{#mobile-current-native-boundary}"
check_contains docs/plan/08_ui_design_03_mobile.md "**Current Native Boundary**"
check_contains docs/plan/08_ui_design_03_mobile.md "**Post-Gate Target**"
check_contains docs/plan/08_ui_design_03_mobile.md "post-gate normative target"
check_contains docs/plan/08_ui_design_03_mobile.md "Tauri v2 Mobile packaging"
check_contains docs/plan/08_ui_design_03_mobile.md "native-packaging"
check_contains docs/plan/08_ui_design_03_mobile.md "{#mobile-packaging-scaffold}"
check_contains docs/plan/08_ui_design_03_mobile.md "{#mobile-packaging-dependency-gate-decision}"
check_contains docs/plan/08_ui_design_03_mobile.md "Mobile native adapter **MUST NOT**"
check_contains docs/plan/08_ui_design_03_mobile.md "service readiness/offline"
check_contains docs/plan/08_ui_design_03_mobile.md "loopback/IPC endpoint"
check_contains docs/plan/08_ui_design_03_mobile.md "{#mobile-native-adapter-contract}"
check_contains docs/plan/08_ui_design_03_mobile.md "NativeEndpointReady { http_base, ws_base, node_role, session_bound }"
check_contains docs/plan/08_ui_design_03_mobile.md "BackgroundSuspended"
check_contains docs/plan/08_ui_design_03_mobile.md "ForegroundReprobe"
check_contains docs/plan/08_ui_design_03_mobile.md "writer_ready(repo_id, scope_nonce)"
check_contains docs/plan/08_ui_design_03_mobile.md "safe-area、keyboard、foreground/background、network online/offline"

check_contains docs/plan/14_tech_stack.md "{#native-packaging-dependency-gate}"
check_contains docs/plan/14_tech_stack.md "native-packaging"
check_contains docs/plan/14_tech_stack.md "CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY"
check_contains docs/plan/14_tech_stack.md "不得进入 workspace root"
check_contains docs/plan/14_tech_stack.md "Desktop packaging scaffold"
check_contains docs/plan/14_tech_stack.md "Mobile packaging scaffold"

check_contains docs/report/README.md "## Current Baselines"
check_contains docs/report/README.md "| Native shell |"
check_contains docs/report/README.md "native-shell-baseline-"
check_not_contains docs/report/next-tasks.md "Desktop / Mobile native adapter plan | P3-10"
check_contains docs/features/operations/tech_stack_platform_release_channel.md "native::adapter_boundary"
check_contains docs/features/operations/tech_stack_platform_release_channel.md "native::service_recovery"
check_contains docs/overview/architecture-code.lisp "native::adapter_boundary"
check_contains docs/overview/architecture-code.lisp "native::service_recovery"
check_contains docs/overview/architecture-doc.lisp "native::adapter_boundary"
check_contains docs/overview/architecture-doc.lisp "native::service_recovery"

echo "native-track-boundary-check: ok"
