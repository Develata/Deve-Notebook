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

check_default_desktop_tree_excludes_tauri() {
  if cargo tree --locked -p deve_desktop --no-default-features | rg -q '(^| )tauri v'; then
    fail "default desktop dependency tree must remain no-Tauri"
  fi
}

check_desktop_native_tree_includes_runtime_surface() {
  local tree
  tree="$(cargo tree --locked -p deve_desktop --features native-packaging)"
  rg -q '(^| )tauri v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tauri"
  rg -q '(^| )tauri-build v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tauri-build"
  rg -q '(^| )tray-icon v' <<<"$tree" \
    || fail "desktop native-packaging tree must include tray-icon"
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
