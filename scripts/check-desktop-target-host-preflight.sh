#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED:-0}"
TARGETS="${DEVE_DESKTOP_TARGET_HOSTS:-macos,windows}"
NO_SIGN="${DEVE_DESKTOP_PACKAGE_NO_SIGN:-0}"

fail() {
  echo "desktop-target-host-preflight-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

is_windows_host() {
  [[ "${OS:-}" == "Windows_NT" ]] && return 0
  [[ "$(host_os)" == MINGW* || "$(host_os)" == MSYS* || "$(host_os)" == CYGWIN* ]]
}

target_enabled() {
  local target="$1"
  IFS=',' read -ra parts <<<"$TARGETS"
  for part in "${parts[@]}"; do
    [[ "${part//[[:space:]]/}" == "$target" ]] && return 0
  done
  return 1
}

validate_targets() {
  local part target has_target=0
  IFS=',' read -ra parts <<<"$TARGETS"
  for part in "${parts[@]}"; do
    target="${part//[[:space:]]/}"
    case "$target" in
      macos|windows)
        has_target=1
        ;;
      "")
        fail "DEVE_DESKTOP_TARGET_HOSTS must list macos or windows"
        ;;
      *)
        fail "DEVE_DESKTOP_TARGET_HOSTS must list only macos or windows; invalid target: $target"
        ;;
    esac
  done
  [[ "$has_target" == "1" ]] || fail "DEVE_DESKTOP_TARGET_HOSTS must list macos or windows"
}

hard_missing=()
target_missing=()

require_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || hard_missing+=("$path")
}

diagnose_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || target_missing+=("$path")
}

diagnose_command() {
  local label="$1"
  shift
  "$@" >/dev/null 2>&1 || target_missing+=("$label")
}

diagnose_any_command() {
  local label="$1"
  shift
  local command_name
  for command_name in "$@"; do
    command -v "$command_name" >/dev/null 2>&1 && return 0
  done
  target_missing+=("$label")
}

has_visual_studio_build_tools() {
  local candidate

  for candidate in \
    "C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" \
    "/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"; do
    [[ -x "$candidate" ]] || continue
    "$candidate" \
      -latest \
      -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
      -property installationPath >/dev/null 2>&1 \
      && return 0
  done
  return 1
}

diagnose_msvc_toolchain() {
  if command -v cl >/dev/null 2>&1 || command -v cl.exe >/dev/null 2>&1; then
    return 0
  fi
  has_visual_studio_build_tools && return 0
  target_missing+=("MSVC cl.exe")
}

diagnose_env() {
  local name="$1"
  [[ -n "${!name:-}" ]] || target_missing+=("env $name")
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

validate_targets
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

require_file "apps/desktop/tauri.conf.json"
require_file "apps/desktop/src/main.rs"
require_file "apps/desktop/build.rs"
require_file "apps/desktop/icons/icon.png"

if ((${#hard_missing[@]} > 0)); then
  for item in "${hard_missing[@]}"; do
    echo "desktop-target-host-preflight-check: invalid $item" >&2
  done
  fail "desktop shell/package boundary is not in the expected preflight state"
fi

run cargo check --locked -p deve_desktop --no-default-features
run cargo check --locked -p deve_desktop --features native-packaging
run cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture

echo "desktop-target-host-preflight-check: host_os=$(host_os)"
echo "desktop-target-host-preflight-check: targets=$TARGETS"

diagnose_file "apps/web/dist/index.html"
diagnose_command "cargo tauri CLI" cargo tauri --version

if target_enabled macos; then
  if [[ "$(host_os)" != "Darwin" ]]; then
    target_missing+=("macOS target-host requires Darwin")
  else
    diagnose_command "xcodebuild" xcodebuild -version
    diagnose_command "xcrun" xcrun --version
    diagnose_rust_target "aarch64-apple-darwin"
    diagnose_rust_target "x86_64-apple-darwin"
    if [[ "$NO_SIGN" != "1" ]]; then
      diagnose_env "APPLE_SIGNING_IDENTITY"
      diagnose_env "APPLE_PROVIDER_SHORT_NAME"
    fi
  fi
fi

if target_enabled windows; then
  if ! is_windows_host; then
    target_missing+=("Windows target-host requires Windows")
  else
    diagnose_any_command "PowerShell" pwsh powershell powershell.exe
    diagnose_msvc_toolchain
    diagnose_any_command "WiX Toolset" wix wix.exe candle candle.exe
    diagnose_any_command "NSIS makensis" makensis makensis.exe
    diagnose_rust_target "x86_64-pc-windows-msvc"
    diagnose_rust_target "aarch64-pc-windows-msvc"
  fi
fi

if ((${#target_missing[@]} > 0)); then
  for item in "${target_missing[@]}"; do
    echo "desktop-target-host-preflight-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "Desktop target-host prerequisites are incomplete"
  fi
  echo "desktop-target-host-preflight-check: skip target-host package build; set DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 on macOS/Windows to require prerequisites"
  echo "desktop-target-host-preflight-check: use DEVE_DESKTOP_TARGET_HOSTS=macos or windows to narrow target diagnostics"
  echo "desktop-target-host-preflight-check: ok"
  exit 0
fi

echo "desktop-target-host-preflight-check: prerequisites present; run scripts/check-desktop-platform-package-build.sh on this target host to build packages"
echo "desktop-target-host-preflight-check: ok"
