#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED:-0}"
BUNDLES="${DEVE_DESKTOP_PACKAGE_BUNDLES:-}"

fail() {
  echo "desktop-platform-package-build-check: $*" >&2
  exit 1
}

run() {
  echo "+ $*"
  "$@"
}

host_os() {
  uname -s 2>/dev/null || printf 'unknown'
}

missing=()

require_file() {
  local path="$1"
  [[ -f "$ROOT_DIR/$path" ]] || missing+=("$path")
}

require_command() {
  local label="$1"
  shift
  "$@" >/dev/null 2>&1 || missing+=("$label")
}

requires_bundle() {
  local bundle="$1"
  [[ -z "$BUNDLES" ]] && return 0
  [[ ",${BUNDLES// /,}," == *",$bundle,"* ]]
}

run "$ROOT_DIR/scripts/check-desktop-package-preflight.sh"

echo "desktop-platform-package-build-check: host_os=$(host_os)"

require_file "apps/desktop/src/main.rs"
require_file "apps/desktop/build.rs"
require_file "apps/web/dist/index.html"
require_command "cargo tauri CLI" cargo tauri --version
if [[ "$REQUIRED" == "1" ]] && requires_bundle "appimage"; then
  require_command "pkg-config librsvg-2.0 (install librsvg2-dev)" pkg-config --exists librsvg-2.0
fi

if ((${#missing[@]} > 0)); then
  for item in "${missing[@]}"; do
    echo "desktop-platform-package-build-check: missing $item" >&2
  done
  if [[ "$REQUIRED" == "1" ]]; then
    fail "platform package build prerequisites are incomplete"
  fi
  echo "desktop-platform-package-build-check: skip package build; set DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 on a target host to require it"
  echo "desktop-platform-package-build-check: ok"
  exit 0
fi

if [[ "$REQUIRED" != "1" ]]; then
  echo "desktop-platform-package-build-check: package prerequisites present; set DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 to run cargo tauri build"
  echo "desktop-platform-package-build-check: set DEVE_DESKTOP_PACKAGE_BUNDLES=deb,rpm to verify a target-host subset"
  echo "desktop-platform-package-build-check: ok"
  exit 0
fi

(
  cd "$ROOT_DIR/apps/desktop"
  if [[ -n "$BUNDLES" ]]; then
    run cargo tauri build --ci --bundles "$BUNDLES"
  else
    run cargo tauri build --ci
  fi
)

echo "desktop-platform-package-build-check: ok"
