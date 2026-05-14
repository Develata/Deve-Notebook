#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REQUIRED="${DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED:-0}"

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

run "$ROOT_DIR/scripts/check-desktop-package-preflight.sh"

echo "desktop-platform-package-build-check: host_os=$(host_os)"

require_file "apps/desktop/src/main.rs"
require_file "apps/desktop/build.rs"
require_file "apps/web/dist/index.html"
require_command "cargo tauri CLI" cargo tauri --version

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
  echo "desktop-platform-package-build-check: ok"
  exit 0
fi

(
  cd "$ROOT_DIR/apps/desktop"
  run cargo tauri build
)

echo "desktop-platform-package-build-check: ok"
