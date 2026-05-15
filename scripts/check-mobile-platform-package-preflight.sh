#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED:-0}"
TARGETS="${DEVE_MOBILE_PACKAGE_TARGETS:-android,ios}"
ANDROID_PACKAGE_TARGET="${DEVE_MOBILE_ANDROID_PACKAGE_TARGET:-aarch64}"

fail() {
  echo "mobile-platform-package-preflight-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

target_enabled() {
  local target="$1"
  IFS=',' read -ra parts <<<"$TARGETS"
  for part in "${parts[@]}"; do
    [[ "${part//[[:space:]]/}" == "$target" ]] && return 0
  done
  return 1
}

hard_missing=()
target_missing=()

require_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || hard_missing+=("$path")
}

forbid_path() {
  local path="$1"
  [[ ! -e "$ROOT_DIR/$path" ]] || hard_missing+=("forbidden generated/runtime path: $path")
}

diagnose_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || target_missing+=("$path")
}

diagnose_dir_env() {
  local label="$1"
  shift
  local var
  for var in "$@"; do
    local value="${!var:-}"
    if [[ -n "$value" && -d "$value" ]]; then
      return 0
    fi
  done
  target_missing+=("$label")
}

diagnose_command() {
  local label="$1"
  shift
  "$@" >/dev/null 2>&1 || target_missing+=("$label")
}

diagnose_rust_target() {
  local target="$1"
  if command -v rg >/dev/null 2>&1; then
    rustup target list --installed 2>/dev/null | rg -qx "$target" \
      || target_missing+=("rust target $target")
  else
    rustup target list --installed 2>/dev/null | grep -Fx -- "$target" >/dev/null \
    || target_missing+=("rust target $target")
  fi
}

android_rust_target() {
  case "$ANDROID_PACKAGE_TARGET" in
    aarch64) printf 'aarch64-linux-android' ;;
    armv7) printf 'armv7-linux-androideabi' ;;
    i686) printf 'i686-linux-android' ;;
    x86_64) printf 'x86_64-linux-android' ;;
    *) target_missing+=("unsupported Android package target $ANDROID_PACKAGE_TARGET") ;;
  esac
}

run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

require_file "apps/mobile/tauri.conf.json"
require_file "apps/mobile/icons/icon.png"
require_file "apps/mobile/build.rs"
require_file "apps/mobile/src/tauri_entry.rs"
forbid_path "apps/mobile/src/main.rs"
forbid_path "apps/mobile/gen/apple"
forbid_path "apps/mobile/src-tauri"

if ((${#hard_missing[@]} > 0)); then
  for item in "${hard_missing[@]}"; do
    echo "mobile-platform-package-preflight-check: invalid $item" >&2
  done
  fail "mobile shell/package boundary is not in the expected preflight state"
fi

run cargo check --locked -p deve_mobile --no-default-features
run cargo check --locked -p deve_mobile --features native-packaging
run cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture

echo "mobile-platform-package-preflight-check: host_os=$(host_os)"
echo "mobile-platform-package-preflight-check: targets=$TARGETS"

diagnose_file "apps/web/dist/index.html"
diagnose_command "cargo tauri CLI" cargo tauri --version

if cargo tauri --version >/dev/null 2>&1; then
  if target_enabled android; then
    diagnose_command "cargo tauri android subcommand" cargo tauri android --help
  fi
  if target_enabled ios; then
    diagnose_command "cargo tauri ios subcommand" cargo tauri ios --help
  fi
fi

if target_enabled android; then
  diagnose_command "java" java -version
  diagnose_command "javac" javac -version
  diagnose_dir_env "ANDROID_HOME or ANDROID_SDK_ROOT" ANDROID_HOME ANDROID_SDK_ROOT
  diagnose_command "adb" adb --version
  android_target="$(android_rust_target)"
  [[ -n "$android_target" ]] && diagnose_rust_target "$android_target"
fi

if target_enabled ios; then
  if [[ "$(host_os)" != "Darwin" ]]; then
    target_missing+=("iOS target-host requires macOS")
  else
    diagnose_command "xcodebuild" xcodebuild -version
    diagnose_command "xcrun" xcrun --version
    diagnose_rust_target "aarch64-apple-ios"
    diagnose_rust_target "aarch64-apple-ios-sim"
  fi
fi

if ((${#target_missing[@]} > 0)); then
  for item in "${target_missing[@]}"; do
    echo "mobile-platform-package-preflight-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "mobile platform package prerequisites are incomplete"
  fi
  echo "mobile-platform-package-preflight-check: skip package build; set DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 on a target host to require prerequisites"
  echo "mobile-platform-package-preflight-check: use DEVE_MOBILE_PACKAGE_TARGETS=android or ios to narrow target diagnostics"
  echo "mobile-platform-package-preflight-check: ok"
  exit 0
fi

echo "mobile-platform-package-preflight-check: prerequisites present; Android shell package build is allowed only through scripts/check-mobile-android-shell-package-build.sh"
echo "mobile-platform-package-preflight-check: prerequisites present; iOS shell package build is allowed only through scripts/check-mobile-ios-shell-package-build.sh"
echo "mobile-platform-package-preflight-check: ok"
