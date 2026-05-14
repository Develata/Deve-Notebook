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
      if [[ "$cargo_file" == "$ROOT_DIR/apps/desktop/Cargo.toml" ]]; then
        continue
      fi
      fail "native packaging dependency is not allowed yet: ${cargo_file#"$ROOT_DIR"/}"
    fi
  done < <(find "$ROOT_DIR" -path "$ROOT_DIR/target" -prune -o -name Cargo.toml -type f -print)

  local runtime_imports
  runtime_imports="$(rg -n '(^|[^[:alnum:]_])((use[[:space:]]+tauri(::|[[:space:];,{]))|tauri::)' \
    "$ROOT_DIR/apps" "$ROOT_DIR/crates" || true)"
  if [[ -n "$runtime_imports" ]]; then
    while IFS= read -r line; do
      case "$line" in
        "$ROOT_DIR/apps/desktop/src/menu_tray.rs":*) ;;
        "$ROOT_DIR/apps/desktop/src/main.rs":*) ;;
        "$ROOT_DIR/apps/desktop/src/tauri_entry.rs":*) ;;
        *) fail "native packaging runtime import outside desktop shell binding: ${line#"$ROOT_DIR"/}" ;;
      esac
    done <<< "$runtime_imports"
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

check_contains apps/desktop/Cargo.toml 'native-packaging = ["dep:tauri", "dep:tauri-build", "tauri/tray-icon", "tauri/wry"]'
check_contains apps/desktop/Cargo.toml 'tauri = { version = "2.11.1", optional = true, default-features = false }'
check_contains apps/desktop/Cargo.toml 'tauri-build = { version = "2.6.1", optional = true, default-features = false }'
check_contains apps/desktop/build.rs "tauri_build::build()"
check_contains apps/desktop/src/main.rs "run_desktop_tauri_app"
check_contains apps/desktop/tauri.conf.json '"identifier": "dev.deve.notebook"'
check_contains apps/desktop/tauri.conf.json '"productName": "Deve Notebook"'
check_contains apps/desktop/tauri.conf.json '"icon": ["icons/icon.png"]'
[[ -f "$ROOT_DIR/apps/desktop/icons/icon.png" ]] \
  || fail "missing desktop Tauri icon: apps/desktop/icons/icon.png"
check_contains apps/desktop/tauri.conf.json '"createUpdaterArtifacts": false'
check_contains apps/mobile/Cargo.toml "native-packaging = []"
check_not_contains apps/mobile/Cargo.toml "tauri ="
check_not_contains apps/mobile/Cargo.toml "tauri-build ="
check_contains apps/desktop/src/lib.rs "#[cfg(feature = \"native-packaging\")]"
check_contains apps/desktop/src/packaging.rs "runtime_crate: \"tauri\""
check_contains apps/desktop/src/packaging.rs "build_crate: \"tauri-build\""
check_contains apps/desktop/src/packaging.rs "status: \"dependency-spike-open\""
check_contains apps/desktop/src/packaging.rs "DesktopShellPackagingAcceptance"
check_contains apps/desktop/src/packaging.rs "DesktopMenuTraySurface"
check_contains apps/desktop/src/packaging.rs "runtime_entrypoint_declared: true"
check_contains apps/desktop/src/packaging.rs "build_script_declared: true"
check_contains apps/desktop/src/packaging.rs "session_handoff_required_before_writable_ui: true"
check_contains apps/desktop/src/packaging.rs "menu_bar_runtime_declared: true"
check_contains apps/desktop/src/packaging.rs "system_tray_runtime_declared: true"
check_contains apps/desktop/src/packaging.rs "menu_and_tray_runtime_deferred: false"
check_contains apps/desktop/src/packaging.rs "menu_runtime_imported: true"
check_contains apps/desktop/src/packaging.rs "tray_runtime_imported: true"
check_contains apps/desktop/src/packaging.rs "actions_are_ui_intents_only: true"
check_contains apps/desktop/src/packaging.rs "opens_process_runtime: false"
check_contains apps/desktop/src/packaging.rs "opens_authority_write_path: false"
check_contains apps/desktop/src/menu_tray.rs "tauri::menu"
check_contains apps/desktop/src/menu_tray.rs "tauri::tray"
check_contains apps/desktop/src/menu_tray.rs "build_desktop_menu"
check_contains apps/desktop/src/menu_tray.rs "build_desktop_tray_icon"
check_contains apps/desktop/src/menu_tray.rs "resolve_desktop_menu_action_id"
check_contains apps/desktop/src/tauri_entry.rs "tauri::Builder::default()"
check_contains apps/desktop/src/tauri_entry.rs "tauri::generate_context!()"
check_contains apps/desktop/src/tauri_entry.rs "DesktopTauriRuntimeSurface"
check_contains apps/desktop/src/tauri_entry.rs "child_process_runtime_enabled: false"
check_contains apps/desktop/src/tauri_entry.rs "opens_authority_write_path: false"
check_contains apps/desktop/src/packaging.rs "child_process_runtime_enabled: false"
check_contains apps/desktop/src/packaging.rs "release_ready_claimed: false"
check_contains apps/desktop/src/packaging.rs "DesktopPackagingAuthority::Ledger"
check_contains apps/desktop/src/packaging.rs "DesktopPackagingCapability::Installer"
check_contains apps/desktop/src/packaging_test.rs "desktop_packaging_dependency_spike_is_feature_gated"
check_contains apps/desktop/src/packaging_test.rs "desktop_tauri_manifest_declares_shell_metadata_only"
check_contains apps/desktop/src/packaging_test.rs "desktop_menu_tray_runtime_binding_declares_ui_intents_only"
check_contains apps/desktop/src/shell_test/policy.rs "CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY"
check_contains apps/desktop/src/shell_test/policy.rs "CURRENT_NATIVE_PROCESS_ADAPTER_POLICY"
check_contains apps/mobile/src/lib.rs "#[cfg(feature = \"native-packaging\")]"
check_contains apps/mobile/src/packaging.rs "runtime_crate: \"tauri\""
check_contains apps/mobile/src/packaging.rs "build_crate: \"tauri-build\""
check_contains apps/mobile/src/packaging.rs "status: \"planned\""
check_contains apps/mobile/src/packaging.rs "MobilePackagingAuthority::Ledger"
check_contains apps/mobile/src/packaging.rs "MobilePackagingCapability::PermissionBridge"
check_contains apps/mobile/src/packaging.rs "MobilePackagingCapability::StorePackage"
check_contains apps/mobile/src/packaging_test.rs "mobile_packaging_scaffold_is_feature_gated_and_planned"
check_contains apps/mobile/src/shell_test/policy.rs "CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY"
check_contains apps/mobile/src/shell_test/policy.rs "CURRENT_NATIVE_PROCESS_ADAPTER_POLICY"
check_contains crates/core/src/native_adapter/packaging.rs "DesktopDependencySpikeOpen"
check_contains crates/core/src/native_adapter/packaging.rs "desktop_tauri_dependencies_allowed: true"
check_contains crates/core/src/native_adapter/packaging.rs "mobile_tauri_dependencies_allowed: false"
check_contains crates/core/src/native_adapter/process.rs "DeferredUntilPackagingGate"
check_contains crates/core/src/native_adapter/process.rs "child_process_runtime_enabled: false"
check_contains crates/core/src/native_adapter/process.rs "record_probe_timeout"
check_contains crates/core/src/native_adapter/process.rs "record_process_stopped"
check_contains crates/core/src/native_adapter/supervisor.rs "record_process_snapshot"
check_contains crates/core/src/native_adapter/supervisor_test/process_observation.rs "process_snapshot_drives_health_and_session_handoff"
check_contains crates/core/src/native_adapter/supervisor_test/process_observation.rs "process_probe_timeout_snapshot_consumes_retry_budget"
check_contains crates/core/src/native_adapter/supervisor_test/process_observation.rs "process_shutdown_snapshot_enters_restart_path"
check_contains crates/core/src/native_adapter/validation.rs "fn validate_port(field: &'static str, port: &str) -> Result<(), NativeAdapterError>"
check_contains crates/core/src/native_adapter/contract_test.rs "native_endpoint_validation_rejects_invalid_or_zero_ports"
check_contains crates/core/src/native_adapter/contract_test.rs "native_reprobe_before_write_requires_full_runtime_readiness"
check_contains apps/web/src/api/service/tests.rs "native_runtime_readiness_requires_node_role_writer_and_current_scope"
check_contains apps/web/src/api/native_bootstrap/tests.rs "rejects_native_bootstrap_with_invalid_or_zero_port"
check_contains apps/web/src/api/service.rs "begin_foreground_reprobe"
check_contains apps/web/src/api/service.rs "native_runtime_readiness_for_untracked"
check_contains apps/web/src/api/connection_role.rs "NODE_ROLE_PROBE_RETRIES"
check_contains apps/web/src/api/connection_role.rs "NODE_ROLE_PROBE_TIMEOUT_MS"
check_contains apps/web/src/api/connection_role.rs "probe_node_role_summary_for_http_base"
check_contains apps/web/src/api/connection_role.rs "strip_prefix(\"wss://\")"
check_contains apps/web/src/api/connection_role/tests.rs "ws_url_to_http_base_only_rewrites_leading_scheme_and_ws_suffix"
check_contains apps/web/src/api/connection_role/tests.rs "stale_node_role_probe_results_do_not_mutate_current_connection"
check_contains apps/web/src/hooks/use_core/effects/handshake/lifecycle.rs "foreground_reprobe_resets_stale_writer_scope_and_node_role"
check_contains apps/desktop/src/shell.rs "observe_process_snapshot"
check_contains apps/desktop/src/shell_recovery_test/process_observation.rs "desktop_probe_timeout_observation_uses_process_snapshot"
check_contains apps/desktop/src/shell_recovery_test/process_observation.rs "desktop_process_shutdown_observation_uses_process_snapshot"
check_contains apps/desktop/src/shell_test/lifecycle.rs "missing_node_role"
check_contains apps/mobile/src/shell.rs "observe_process_snapshot"
check_contains apps/mobile/src/shell_recovery_test/process_observation.rs "mobile_probe_timeout_observation_uses_process_snapshot"
check_contains apps/mobile/src/shell_recovery_test/process_observation.rs "mobile_process_shutdown_observation_uses_process_snapshot"
check_contains apps/mobile/src/shell_test/lifecycle.rs "missing_node_role"
check_contains apps/web/src/hooks/use_core/write_gate/logic.rs "node_role_readable"
check_contains apps/web/src/hooks/use_core/write_gate/tests.rs "repo_write_gate_requires_node_role_readable"
check_contains apps/web/src/hooks/use_core/write_gate/tests.rs "repo_source_control_read_gate_reports_node_role_probe_failure_before_read_only"
check_contains apps/web/src/hooks/use_core/write_gate/tests.rs "repo_source_control_read_gate_requires_node_role_for_local_refresh"
check_contains apps/web/src/hooks/use_core/write_gate/tests.rs "repo_source_control_read_gate_allows_remote_branch_reads_without_node_role"
check_contains apps/web/src/hooks/use_core/write_gate/tests.rs "repo_write_gate_reports_node_role_probe_failure_before_snapshot_loading"
check_contains apps/web/src/hooks/use_core/status_summary/tests.rs "reports_repo_handshake_until_node_role_is_readable"
check_contains apps/web/src/hooks/use_core/status_summary/tests.rs "reports_native_reprobe_when_node_role_probe_failed"
check_contains apps/web/src/hooks/use_core/status_summary/tests.rs "reports_native_reprobe_before_snapshot_loading_when_node_role_probe_failed"
check_contains apps/web/src/editor/delta_input_gate.rs "blocks_delta_when_runtime_write_gate_blocks"
check_contains apps/web/src/editor/mod.rs "editor_read_only_gate_blocks_native_runtime_write_gate"
check_contains apps/web/src/editor/hook_playback.rs "playback_read_only_gate_blocks_native_runtime_write_gate"
check_contains apps/web/src/editor/delta_input.rs "repo_write_block_untracked"
check_contains apps/web/src/editor/sync/history_resend.rs "repo_write_block_untracked"
check_contains apps/web/src/editor/sync/dispatch_payload.rs "write_ready_resend_blocks_when_native_runtime_readiness_fails"
check_contains apps/web/src/components/chat/actions/apply.rs "repo_write_block_untracked"
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
check_contains docs/plan/14_tech_stack.md "Desktop packaging dependency spike"
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
