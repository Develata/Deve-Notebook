#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"
REQUIRED="${DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED:-0}"
TARGETS="${DEVE_DESKTOP_SIGNING_TARGETS:-macos,windows}"

# This gate validates prerequisite shape only. It must not sign artifacts,
# notarize artifacts, upload releases, or open native process/authority paths.

fail() {
  echo "desktop-signing-preflight-check: $*" >&2
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
  local part
  for part in "${parts[@]}"; do
    [[ "${part//[[:space:]]/}" == "$target" ]] && return 0
  done
  return 1
}

missing=()

diagnose_env() {
  local name="$1"
  [[ -n "${!name:-}" ]] || missing+=("env $name")
}

diagnose_command_name() {
  local label="$1"
  shift
  local command_name
  for command_name in "$@"; do
    command -v "$command_name" >/dev/null 2>&1 && return 0
  done
  missing+=("$label")
}

diagnose_file_or_env() {
  local label="$1"
  local path_var="$2"
  local base64_var="$3"
  local path_value="${!path_var:-}"
  local base64_value="${!base64_var:-}"

  if [[ -n "$path_value" ]]; then
    [[ -f "$path_value" || -f "$ROOT_DIR/$path_value" ]] && return 0
    missing+=("$label path from $path_var")
    return 0
  fi
  [[ -n "$base64_value" ]] && return 0
  missing+=("$label via $path_var or $base64_var")
}

has_apple_notarization_credentials() {
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    return 0
  fi
  if [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_KEY_ID:-}" && -n "${APPLE_API_ISSUER:-}" ]]; then
    return 0
  fi
  return 1
}

diagnose_apple_notarization_credentials() {
  has_apple_notarization_credentials && return 0
  missing+=("Apple notarization credentials: APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID or APPLE_API_KEY/APPLE_API_KEY_ID/APPLE_API_ISSUER")
}

run_deve_baseline "$ROOT_DIR" "desktop-signing-preflight" "desktop-signing-preflight-check"
run "$ROOT_DIR/scripts/check-native-track-boundary.sh"

echo "desktop-signing-preflight-check: host_os=$(host_os)"
echo "desktop-signing-preflight-check: targets=$TARGETS"

if target_enabled macos; then
  if [[ "$(host_os)" != "Darwin" ]]; then
    missing+=("macOS signing requires Darwin target host")
  else
    diagnose_command_name "codesign" codesign
    diagnose_command_name "xcrun" xcrun
    xcrun notarytool --help >/dev/null 2>&1 || missing+=("xcrun notarytool")
  fi
  diagnose_env "APPLE_SIGNING_IDENTITY"
  diagnose_env "APPLE_PROVIDER_SHORT_NAME"
  diagnose_apple_notarization_credentials
fi

if target_enabled windows; then
  if ! is_windows_host; then
    missing+=("Windows signing requires Windows target host")
  else
    diagnose_command_name "signtool" signtool signtool.exe
  fi
  diagnose_file_or_env "Windows signing certificate" WINDOWS_SIGNING_CERT_PATH WINDOWS_SIGNING_CERT_BASE64
  diagnose_env "WINDOWS_SIGNING_CERT_PASSWORD"
fi

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-signing-preflight-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "Desktop signing prerequisites are incomplete"
  fi
  echo "desktop-signing-preflight-check: signing/notarization not executed; set DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED=1 on a signing target host to require prerequisites"
  echo "desktop-signing-preflight-check: use DEVE_DESKTOP_SIGNING_TARGETS=macos or windows to narrow diagnostics"
  echo "desktop-signing-preflight-check: ok"
  exit 0
fi

echo "desktop-signing-preflight-check: prerequisites present; signing/notarization remains a separate explicit release step"
echo "desktop-signing-preflight-check: ok"
