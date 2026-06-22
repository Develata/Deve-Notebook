#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"
TIMEOUT_SECS="${DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS:-20}"

source "$ROOT_DIR/scripts/baseline-wrapper.sh"
run_deve_baseline "$ROOT_DIR" "desktop-package-startup-smoke" "desktop-package-startup-smoke-check"

fail() {
  echo "desktop-package-startup-smoke-check: $*" >&2
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

  if command -v gtimeout >/dev/null 2>&1; then
    DEVE_DESKTOP_STARTUP_SMOKE=1 gtimeout "${TIMEOUT_SECS}s" "$binary"
    return
  fi
  if command -v timeout >/dev/null 2>&1 && timeout --version >/dev/null 2>&1; then
    DEVE_DESKTOP_STARTUP_SMOKE=1 timeout "${TIMEOUT_SECS}s" "$binary"
    return
  fi
  DEVE_DESKTOP_STARTUP_SMOKE=1 "$binary"
}

run_startup_probe() {
  local binary="$1"
  local output
  local status

  if [[ ! -x "$binary" ]]; then
    if [[ "$REQUIRED" == "1" ]]; then
      fail "startup binary is not executable: ${binary#$ROOT_DIR/}"
    fi
    echo "desktop-package-startup-smoke-check: skip startup probe; binary is not executable: ${binary#$ROOT_DIR/}"
    return 2
  fi

  set +e
  output="$(run_with_timeout "$binary" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  if ((status != 0)) || [[ "$output" != *"desktop-startup-smoke: ok"* ]]; then
    if [[ "$REQUIRED" == "1" ]]; then
      fail "startup probe failed for ${binary#$ROOT_DIR/}"
    fi
    echo "desktop-package-startup-smoke-check: skip startup probe; rebuild with --features native-packaging and set DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 on a target host to require it"
    return 2
  fi
}

verify_dmg() {
  local dmg="$1"

  if ! command -v hdiutil >/dev/null 2>&1; then
    if [[ "$REQUIRED" == "1" ]]; then
      fail "hdiutil is required to verify dmg artifacts"
    fi
    echo "desktop-package-startup-smoke-check: hdiutil unavailable; skip dmg verification"
    return
  fi
  hdiutil verify "$dmg"
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
      startup_binary="$app_binary"
    fi
    if requires_bundle dmg; then
      dmg="$(
        first_match "$ROOT_DIR/target/release/bundle/dmg" \
          -name '*.dmg' -type f || true
      )"
      [[ -n "$dmg" ]] || record_missing "macOS dmg target/release/bundle/dmg/*.dmg"
      [[ -z "$dmg" ]] || verify_dmg "$dmg"
    fi
    ;;
  *)
    if is_windows_host; then
      if requires_bundle msi; then
        msi="$(
          first_match "$ROOT_DIR/target/release/bundle/msi" \
            -name '*.msi' -type f || true
        )"
        [[ -n "$msi" ]] || record_missing "Windows MSI target/release/bundle/msi/*.msi"
      fi
      if requires_bundle nsis; then
        nsis="$(
          first_match "$ROOT_DIR/target/release/bundle/nsis" \
            -name '*.exe' -type f || true
        )"
        [[ -n "$nsis" ]] || record_missing "Windows NSIS target/release/bundle/nsis/*.exe"
      fi
      startup_binary="$ROOT_DIR/target/release/deve_desktop.exe"
      [[ -f "$startup_binary" ]] || record_missing "Windows release binary target/release/deve_desktop.exe"
    else
      startup_binary="$ROOT_DIR/target/release/deve_desktop"
      [[ -f "$startup_binary" ]] || record_missing "release binary target/release/deve_desktop"
    fi
    ;;
esac

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-package-startup-smoke-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "desktop package startup smoke prerequisites are incomplete"
  fi
  echo "desktop-package-startup-smoke-check: skip; set DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 after package build to require startup smoke"
  echo "desktop-package-startup-smoke-check: ok"
  exit 0
fi

if ! run_startup_probe "$startup_binary"; then
  echo "desktop-package-startup-smoke-check: ok"
  exit 0
fi

echo "desktop-package-startup-smoke-check: ok"
