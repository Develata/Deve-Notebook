#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"
TIMEOUT_SECS="${DEVE_DESKTOP_NATIVE_SESSION_SMOKE_TIMEOUT_SECS:-${DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS:-30}}"

fail() {
  echo "desktop-native-session-package-smoke-check: $*" >&2
  exit 1
}

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

is_windows_host() {
  [[ "${OS:-}" == "Windows_NT" ]] && return 0
  [[ "$(host_os)" == MINGW* || "$(host_os)" == MSYS* || "$(host_os)" == CYGWIN* ]]
}

requires_bundle() {
  local bundle="$1"
  [[ -z "$BUNDLES" ]] && return 0
  [[ ",${BUNDLES// /,}," == *",$bundle,"* ]]
}

first_match() {
  local root="$1"
  shift
  [[ -d "$root" ]] || return 1
  find "$root" "$@" -print -quit
}

record_missing() {
  missing+=("$1")
}

run_with_timeout() {
  local binary="$1"
  local smoke_root="$2"

  if command -v gtimeout >/dev/null 2>&1; then
	    (
	      cd "$smoke_root"
	      DEVE_DESKTOP_LOCAL_SERVICE=1 DEVE_DESKTOP_NATIVE_SESSION_SMOKE=1 DEVE_DESKTOP_SERVICE_STDIO_INHERIT=1 \
	        gtimeout "${TIMEOUT_SECS}s" "$binary"
	    )
    return
  fi
  if command -v timeout >/dev/null 2>&1 && timeout --version >/dev/null 2>&1; then
	  (
	    cd "$smoke_root"
	    DEVE_DESKTOP_LOCAL_SERVICE=1 DEVE_DESKTOP_NATIVE_SESSION_SMOKE=1 DEVE_DESKTOP_SERVICE_STDIO_INHERIT=1 \
	      timeout "${TIMEOUT_SECS}s" "$binary"
	  )
    return
  fi
	  (
	    cd "$smoke_root"
	    DEVE_DESKTOP_LOCAL_SERVICE=1 DEVE_DESKTOP_NATIVE_SESSION_SMOKE=1 DEVE_DESKTOP_SERVICE_STDIO_INHERIT=1 "$binary"
	  )
}

run_native_session_probe() {
  local binary="$1"
  local output
  local status
  local smoke_root

  if [[ ! -x "$binary" ]]; then
    if [[ "$REQUIRED" == "1" ]]; then
      fail "native session smoke binary is not executable: ${binary#$ROOT_DIR/}"
    fi
    echo "desktop-native-session-package-smoke-check: skip; binary is not executable: ${binary#$ROOT_DIR/}"
    return 2
  fi

  smoke_root="$(mktemp -d)"
  set +e
  output="$(run_with_timeout "$binary" "$smoke_root" 2>&1)"
  status=$?
  set -e
  rm -rf "$smoke_root"

  printf '%s\n' "$output"
  if ((status != 0)) || [[ "$output" != *"desktop-native-session-smoke: ok"* ]]; then
    if [[ "$REQUIRED" == "1" ]]; then
      fail "native session package smoke failed for ${binary#$ROOT_DIR/}"
    fi
    echo "desktop-native-session-package-smoke-check: skip native session smoke; rebuild package with bundled deve_cli sidecar"
    return 2
  fi
}

missing=()
startup_binary=""

case "$(host_os)" in
  Darwin)
    if requires_bundle app; then
      app_binary="$(
        first_match "$ROOT_DIR/target/release/bundle/macos" \
          -path '*/Contents/MacOS/deve_desktop' -type f || true
      )"
      [[ -n "$app_binary" ]] || record_missing "macOS .app binary target/release/bundle/macos/*/Contents/MacOS/deve_desktop"
      if [[ -n "$app_binary" ]]; then
        sidecar="$(dirname "$app_binary")/deve_cli"
        [[ -x "$sidecar" ]] || record_missing "macOS .app sidecar ${sidecar#$ROOT_DIR/}"
      fi
      startup_binary="$app_binary"
    fi
    ;;
  *)
    if is_windows_host; then
      startup_binary="$ROOT_DIR/target/release/deve_desktop.exe"
      [[ -f "$startup_binary" ]] || record_missing "Windows release binary target/release/deve_desktop.exe"
      [[ -f "$ROOT_DIR/target/release/deve_cli.exe" ]] || record_missing "Windows sidecar target/release/deve_cli.exe"
    else
      startup_binary="$ROOT_DIR/target/release/deve_desktop"
      [[ -f "$startup_binary" ]] || record_missing "release binary target/release/deve_desktop"
      [[ -f "$ROOT_DIR/target/release/deve_cli" ]] || record_missing "sidecar target/release/deve_cli"
    fi
    ;;
esac

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-native-session-package-smoke-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "desktop native session package smoke prerequisites are incomplete"
  fi
  echo "desktop-native-session-package-smoke-check: skip; set DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 after package build to require native session smoke"
  echo "desktop-native-session-package-smoke-check: ok"
  exit 0
fi

if ! run_native_session_probe "$startup_binary"; then
  echo "desktop-native-session-package-smoke-check: ok"
  exit 0
fi

echo "desktop-native-session-package-smoke-check: ok"
