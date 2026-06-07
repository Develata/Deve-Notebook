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
  CARGO_TARGET_ARG="${CARGO_TARGET_DIR:-target/native-packaging-gate}"
}

run_cargo() {
  "$CARGO_BIN" "$@"
}

run_cargo_target() {
  local subcommand="$1"
  shift
  "$CARGO_BIN" "$subcommand" --target-dir "$CARGO_TARGET_ARG" "$@"
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
  output="$(run_cargo tree --locked "$@")"
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
select_cargo_bin
configure_cargo_target_dir
bash -n "$ROOT_DIR/scripts/check-desktop-native-session-package-smoke.sh"
path_contains_regex 'explicit opt-in' "$ROOT_DIR/docs/plan/17_tech_stack.md" \
  || fail "native authority opt-in boundary must be documented in tech stack plan"
path_contains_regex 'Native authority 只在 explicit opt-in 下可用' "$ROOT_DIR/docs/features/15_release.md" \
  || fail "release feature must keep native authority as explicit opt-in"
path_contains_regex 'DEVE_NATIVE_AUTHORITY=1' "$ROOT_DIR/docs/dev-runbook.md" \
  || fail "runbook must document native authority opt-in env"
check_desktop_tauri_lock_entries
check_mobile_tauri_lock_entries
check_default_desktop_tree_excludes_tauri
check_default_mobile_tree_excludes_tauri
check_desktop_feature_tree_includes_tauri
check_mobile_feature_tree_includes_tauri

run_cargo_target check --locked -p deve_desktop --no-default-features
run_cargo_target check --locked -p deve_mobile --no-default-features
run_cargo_target check --locked -p deve_mobile --features native-packaging
run_cargo_target test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture
run_cargo_target test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture
run_cargo_target test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture
run_cargo_target test --locked -p deve_desktop --features native-packaging tauri_bootstrap -- --nocapture
run_cargo_target test --locked -p deve_desktop --features native-packaging menu_tray -- --nocapture
run_cargo_target test --locked -p deve_desktop --features native-packaging packaging -- --nocapture
run_cargo_target test --locked -p deve_mobile --features native-packaging packaging -- --nocapture
run_cargo_target test --locked -p deve_cli native_session -- --nocapture
"$ROOT_DIR/scripts/check-desktop-native-session-package-smoke.sh"

check_desktop_tauri_lock_entries
check_mobile_tauri_lock_entries

echo "native-packaging-gate-check: ok"
