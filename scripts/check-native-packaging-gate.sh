#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-packaging-gate-check: $*" >&2
  exit 1
}

check_desktop_tauri_lock_entries() {
  rg -q 'name = "tauri"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging dependency spike must lock tauri"
  rg -q 'name = "tauri-build"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging dependency spike must lock tauri-build"
}

check_default_desktop_tree_excludes_tauri() {
  if cargo tree --locked -p deve_desktop --no-default-features | rg -q '(^| )tauri v'; then
    fail "default desktop dependency tree must remain no-Tauri"
  fi
}

check_desktop_feature_tree_includes_tauri() {
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tauri v' \
    || fail "desktop native-packaging feature must include tauri"
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tauri-build v' \
    || fail "desktop native-packaging feature must include tauri-build"
}

"$ROOT_DIR/scripts/check-native-track-boundary.sh"
check_desktop_tauri_lock_entries
check_default_desktop_tree_excludes_tauri
check_desktop_feature_tree_includes_tauri

cargo check --locked -p deve_desktop --no-default-features
cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture

check_desktop_tauri_lock_entries

echo "native-packaging-gate-check: ok"
