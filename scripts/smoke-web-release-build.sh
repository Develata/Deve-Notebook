#!/usr/bin/env bash
set -euo pipefail

# CMD-007A Web release build smoke.
# Normalizes environment quirks observed in WSL/Trunk:
# - Trunk 0.21 expects NO_COLOR to be a bool-like value, not "1".
# - Trunk's Tailwind pipeline may emit non-actionable Browserslist DB freshness
#   noise even though this repo does not lock browserslist/caniuse-lite.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

windows_path_to_wsl() {
  local path="$1"
  path="${path//$'\r'/}"
  path="${path//\\//}"
  if [[ "$path" =~ ^([A-Za-z]):/(.*)$ ]]; then
    printf '/mnt/%s/%s\n' "$(printf '%s' "${BASH_REMATCH[1]}" | tr '[:upper:]' '[:lower:]')" "${BASH_REMATCH[2]}"
  else
    printf '%s\n' "$path"
  fi
}

repo_on_wsl_windows_mount() {
  [[ "$ROOT_DIR" =~ ^/mnt/[A-Za-z] ]]
}

web_node_modules_has_windows_esbuild_only() {
  [[ -d "$ROOT_DIR/apps/web/node_modules/@esbuild/win32-x64" ]] \
    && [[ ! -d "$ROOT_DIR/apps/web/node_modules/@esbuild/linux-x64" ]]
}

resolve_windows_trunk_bin() {
  local candidate
  if ! command -v where.exe >/dev/null 2>&1; then
    return 1
  fi
  candidate="$(where.exe trunk 2>/dev/null | head -n1 || true)"
  if [[ -z "$candidate" ]]; then
    return 1
  fi
  windows_path_to_wsl "$candidate"
}

resolve_trunk_bin() {
  local candidate
  if [[ -n "${DEVE_TRUNK_BIN:-}" ]]; then
    printf '%s\n' "$DEVE_TRUNK_BIN"
    return
  fi

  if repo_on_wsl_windows_mount && web_node_modules_has_windows_esbuild_only; then
    candidate="$(resolve_windows_trunk_bin || true)"
    if [[ -n "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return
    fi
  fi

  for candidate in trunk trunk.exe; do
    if command -v "$candidate" >/dev/null 2>&1; then
      command -v "$candidate"
      return
    fi
  done
  candidate="$(resolve_windows_trunk_bin || true)"
  if [[ -n "$candidate" ]]; then
    printf '%s\n' "$candidate"
    return
  fi
  printf 'trunk\n'
}

TRUNK_BIN="$(resolve_trunk_bin)"

export NO_COLOR="${NO_COLOR:-true}"
if [[ "$NO_COLOR" == "1" ]]; then
  export NO_COLOR=true
fi
export BROWSERSLIST_IGNORE_OLD_DATA="${BROWSERSLIST_IGNORE_OLD_DATA:-true}"

cd "$ROOT_DIR/apps/web"
echo "web-release-build-smoke: trunk=$TRUNK_BIN"
"$TRUNK_BIN" build --release
