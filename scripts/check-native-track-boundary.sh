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
  while IFS= read -r cargo_file; do
    if rg -iq '^[[:space:]]*(tauri|tauri-build)[[:space:]]*=' "$cargo_file"; then
      fail "native packaging dependency is not allowed yet: ${cargo_file#$ROOT_DIR/}"
    fi
  done < <(find "$ROOT_DIR" -path "$ROOT_DIR/target" -prune -o -name Cargo.toml -type f -print)

  if rg -n '(^|[^[:alnum:]_])(use[[:space:]]+tauri|tauri::)' \
    "$ROOT_DIR/apps" "$ROOT_DIR/crates" >/dev/null; then
    fail "native packaging runtime imports must stay absent until the packaging gate opens"
  fi
}

check_contains Cargo.toml '"apps/desktop"'
check_contains Cargo.toml '"apps/mobile"'

check_contains apps/desktop/Cargo.toml "native-packaging = []"
check_contains apps/mobile/Cargo.toml "native-packaging = []"
check_no_packaging_dependency_leak

check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-current-native-boundary}"
check_contains docs/plan/08_ui_design_02_desktop.md "Tauri v2 native packaging"
check_contains docs/plan/08_ui_design_02_desktop.md "native-packaging"
check_contains docs/plan/08_ui_design_02_desktop.md "Native adapter **MUST NOT**"
check_contains docs/plan/08_ui_design_02_desktop.md "service readiness/offline"
check_contains docs/plan/08_ui_design_02_desktop.md "loopback/IPC endpoint"
check_contains docs/plan/08_ui_design_02_desktop.md "{#desktop-native-adapter-contract}"
check_contains docs/plan/08_ui_design_02_desktop.md "NativeEndpointReady { http_base, ws_base, node_role, session_bound }"
check_contains docs/plan/08_ui_design_02_desktop.md "writer_ready(repo_id, scope_nonce)"
check_contains docs/plan/08_ui_design_02_desktop.md "09_auth.md#unauthorized-disconnected-ui"
check_contains docs/plan/08_ui_design_02_desktop.md "native 层不得直接写 ledger/vault/source-control/search index"

check_contains docs/plan/08_ui_design_03_mobile.md "{#mobile-current-native-boundary}"
check_contains docs/plan/08_ui_design_03_mobile.md "Tauri v2 Mobile packaging"
check_contains docs/plan/08_ui_design_03_mobile.md "native-packaging"
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
check_contains docs/plan/14_tech_stack.md "不得进入 workspace root"

check_contains docs/report/next-tasks.md "Native packaging dependency gate"
check_contains docs/report/next-tasks.md "P3-10 Desktop native shell skeleton 已关闭"
check_contains docs/report/next-tasks.md "P3-10 Mobile native shell skeleton 已关闭"
check_not_contains docs/report/next-tasks.md "Desktop / Mobile native adapter plan | P3-10"

echo "native-track-boundary-check: ok"
