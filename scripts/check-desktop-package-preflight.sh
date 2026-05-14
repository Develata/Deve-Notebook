#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "desktop-package-preflight-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

stdin_contains_regex() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -q "$pattern"
  else
    grep -E -- "$pattern" >/dev/null
  fi
}

check_default_desktop_tree_excludes_tauri() {
  if cargo tree --locked -p deve_desktop --no-default-features | stdin_contains_regex '(^| )tauri v'; then
    fail "default desktop dependency tree must remain no-Tauri"
  fi
}

check_desktop_native_tree_includes_runtime_surface() {
  local tree
  tree="$(cargo tree --locked -p deve_desktop --features native-packaging)"
  stdin_contains_regex '(^| )tauri v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tauri"
  stdin_contains_regex '(^| )tauri-build v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tauri-build"
  stdin_contains_regex '(^| )tray-icon v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tray-icon"
  stdin_contains_regex '(^| )tauri-runtime-wry v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tauri-runtime-wry"
}

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"
check_default_desktop_tree_excludes_tauri
check_desktop_native_tree_includes_runtime_surface

run cargo check --locked -p deve_desktop --no-default-features
run cargo test --locked -p deve_desktop --no-default-features -- --nocapture
run cargo check --locked -p deve_desktop --features native-packaging
run cargo test --locked -p deve_desktop --features native-packaging menu_tray -- --nocapture
run cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture

echo "desktop-package-preflight-check: ok"
