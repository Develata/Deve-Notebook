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

check_contains docs/plan/08_ui_design_02_desktop.md "Current Native Boundary (2026-04-28)"
check_contains docs/plan/08_ui_design_02_desktop.md "Tauri v2 native packaging"
check_contains docs/plan/08_ui_design_02_desktop.md "Native adapter **MUST NOT**"
check_contains docs/plan/08_ui_design_02_desktop.md "service readiness/offline"
check_contains docs/plan/08_ui_design_02_desktop.md "loopback/IPC endpoint"

check_contains docs/plan/08_ui_design_03_mobile.md "Current Native Boundary (2026-04-28)"
check_contains docs/plan/08_ui_design_03_mobile.md "Tauri v2 Mobile packaging"
check_contains docs/plan/08_ui_design_03_mobile.md "Mobile native adapter **MUST NOT**"
check_contains docs/plan/08_ui_design_03_mobile.md "service readiness/offline"
check_contains docs/plan/08_ui_design_03_mobile.md "loopback/IPC endpoint"

check_contains docs/report/next-tasks.md "P3-10 Desktop / Mobile Native Track"

echo "native-track-boundary-check: ok"
