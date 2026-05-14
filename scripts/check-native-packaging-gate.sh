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
  rg -q 'name = "tray-icon"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging menu/tray binding must lock tray-icon"
  rg -q 'name = "tauri-runtime-wry"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging runtime entrypoint must lock tauri-runtime-wry"
}

check_mobile_tauri_lock_entries() {
  rg -q 'name = "tauri"' "$ROOT_DIR/Cargo.lock" \
    || fail "mobile native-packaging dependency spike must lock tauri"
  rg -q 'name = "tauri-build"' "$ROOT_DIR/Cargo.lock" \
    || fail "mobile native-packaging dependency spike must lock tauri-build"
}

check_default_desktop_tree_excludes_tauri() {
  if cargo tree --locked -p deve_desktop --no-default-features | rg -q '(^| )tauri v'; then
    fail "default desktop dependency tree must remain no-Tauri"
  fi
}

check_default_mobile_tree_excludes_tauri() {
  if cargo tree --locked -p deve_mobile --no-default-features | rg -q '(^| )tauri v'; then
    fail "default mobile dependency tree must remain no-Tauri"
  fi
}

check_desktop_feature_tree_includes_tauri() {
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tauri v' \
    || fail "desktop native-packaging feature must include tauri"
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tauri-build v' \
    || fail "desktop native-packaging feature must include tauri-build"
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tray-icon v' \
    || fail "desktop native-packaging feature must include tray-icon"
  cargo tree --locked -p deve_desktop --features native-packaging | rg -q '(^| )tauri-runtime-wry v' \
    || fail "desktop native-packaging feature must include tauri-runtime-wry"
}

check_mobile_feature_tree_includes_tauri() {
  cargo tree --locked -p deve_mobile --features native-packaging | rg -q '(^| )tauri v' \
    || fail "mobile native-packaging feature must include tauri"
  cargo tree --locked -p deve_mobile --features native-packaging | rg -q '(^| )tauri-build v' \
    || fail "mobile native-packaging feature must include tauri-build"
}

"$ROOT_DIR/scripts/check-native-track-boundary.sh"
check_desktop_tauri_lock_entries
check_mobile_tauri_lock_entries
check_default_desktop_tree_excludes_tauri
check_default_mobile_tree_excludes_tauri
check_desktop_feature_tree_includes_tauri
check_mobile_feature_tree_includes_tauri

cargo check --locked -p deve_desktop --no-default-features
cargo check --locked -p deve_mobile --no-default-features
cargo check --locked -p deve_mobile --features native-packaging
cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging menu_tray -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture

check_desktop_tauri_lock_entries
check_mobile_tauri_lock_entries

echo "native-packaging-gate-check: ok"
