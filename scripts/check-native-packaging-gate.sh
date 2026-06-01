#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-packaging-gate-check: $*" >&2
  exit 1
}

is_windows_bash_host() {
  case "$(uname -s 2>/dev/null || printf 'unknown')" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

path_contains_regex() {
  local pattern="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1 && ! is_windows_bash_host; then
    rg -q "$pattern" "$file"
  else
    grep -E -- "$pattern" "$file" >/dev/null
  fi
}

stdin_contains_regex() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -q "$pattern"
  else
    grep -E -- "$pattern" >/dev/null
  fi
}

cargo_tree_contains() {
  local pattern="$1"
  shift
  local output
  output="$(cargo tree --locked "$@")"
  stdin_contains_regex "$pattern" <<<"$output"
}

check_desktop_tauri_lock_entries() {
  path_contains_regex 'name = "tauri"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging dependency spike must lock tauri"
  path_contains_regex 'name = "tauri-build"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging dependency spike must lock tauri-build"
  path_contains_regex 'name = "tray-icon"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging menu/tray binding must lock tray-icon"
  path_contains_regex 'name = "tauri-runtime-wry"' "$ROOT_DIR/Cargo.lock" \
    || fail "desktop native-packaging runtime entrypoint must lock tauri-runtime-wry"
}

check_mobile_tauri_lock_entries() {
  path_contains_regex 'name = "tauri"' "$ROOT_DIR/Cargo.lock" \
    || fail "mobile native-packaging dependency spike must lock tauri"
  path_contains_regex 'name = "tauri-build"' "$ROOT_DIR/Cargo.lock" \
    || fail "mobile native-packaging dependency spike must lock tauri-build"
  path_contains_regex 'name = "tauri-runtime-wry"' "$ROOT_DIR/Cargo.lock" \
    || fail "mobile native-packaging shell entrypoint must lock tauri-runtime-wry"
}

check_default_desktop_tree_excludes_tauri() {
  if cargo_tree_contains '(^| )tauri v' -p deve_desktop --no-default-features; then
    fail "default desktop dependency tree must remain no-Tauri"
  fi
}

check_default_mobile_tree_excludes_tauri() {
  if cargo_tree_contains '(^| )tauri v' -p deve_mobile --no-default-features; then
    fail "default mobile dependency tree must remain no-Tauri"
  fi
}

check_desktop_feature_tree_includes_tauri() {
  cargo_tree_contains '(^| )tauri v' -p deve_desktop --features native-packaging \
    || fail "desktop native-packaging feature must include tauri"
  cargo_tree_contains '(^| )tauri-build v' -p deve_desktop --features native-packaging \
    || fail "desktop native-packaging feature must include tauri-build"
  cargo_tree_contains '(^| )tray-icon v' -p deve_desktop --features native-packaging \
    || fail "desktop native-packaging feature must include tray-icon"
  cargo_tree_contains '(^| )tauri-runtime-wry v' -p deve_desktop --features native-packaging \
    || fail "desktop native-packaging feature must include tauri-runtime-wry"
}

check_mobile_feature_tree_includes_tauri() {
  cargo_tree_contains '(^| )tauri v' -p deve_mobile --features native-packaging \
    || fail "mobile native-packaging feature must include tauri"
  cargo_tree_contains '(^| )tauri-build v' -p deve_mobile --features native-packaging \
    || fail "mobile native-packaging feature must include tauri-build"
  cargo_tree_contains '(^| )tauri-runtime-wry v' -p deve_mobile --features native-packaging \
    || fail "mobile native-packaging feature must include tauri-runtime-wry"
}

"$ROOT_DIR/scripts/check-native-track-boundary.sh"
bash -n "$ROOT_DIR/scripts/check-desktop-native-session-package-smoke.sh"
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
cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging tauri_bootstrap -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging menu_tray -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_cli native_session -- --nocapture
"$ROOT_DIR/scripts/check-desktop-native-session-package-smoke.sh"

check_desktop_tauri_lock_entries
check_mobile_tauri_lock_entries

echo "native-packaging-gate-check: ok"
