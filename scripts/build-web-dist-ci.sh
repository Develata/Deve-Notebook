#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_TOOL_BIN_DIR="${DEVE_NATIVE_TOOL_BIN_DIR:-$ROOT_DIR/target/native-tools/bin}"

fail() {
  echo "web-dist-ci: $*" >&2
  exit 1
}

resolve_tool() {
  local tool_name="$1"
  shift
  local candidate
  local candidate_path

  for candidate in "$tool_name" "$@"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      candidate_path="$(command -v "$candidate")"
      if "$candidate_path" --version >/dev/null 2>&1; then
        printf '%s\n' "$candidate_path"
        return
      fi
      echo "web-dist-ci: ignored non-runnable $candidate at $candidate_path" >&2
    fi
    if [[ -x "$NATIVE_TOOL_BIN_DIR/$candidate" ]]; then
      candidate_path="$NATIVE_TOOL_BIN_DIR/$candidate"
      if "$candidate_path" --version >/dev/null 2>&1; then
        printf '%s\n' "$candidate_path"
        return
      fi
      echo "web-dist-ci: ignored non-runnable $candidate_path" >&2
    fi
  done

  fail "missing required tool '$tool_name'"
}

print_tool_version() {
  local label="$1"
  local bin="$2"

  echo "web-dist-ci: $label=$bin"
  "$bin" --version
}

export PATH="$NATIVE_TOOL_BIN_DIR:$PATH"

echo "web-dist-ci: os=$(uname -s 2>/dev/null || printf unknown)"
echo "web-dist-ci: arch=$(uname -m 2>/dev/null || printf unknown)"
echo "web-dist-ci: pwd=$ROOT_DIR"
echo "web-dist-ci: native_tool_bin_dir=$NATIVE_TOOL_BIN_DIR"

npm_bin="$(resolve_tool npm npm.cmd)"
trunk_bin="$(resolve_tool trunk trunk.exe)"

print_tool_version npm "$npm_bin"
print_tool_version trunk "$trunk_bin"
if node_bin="$(resolve_tool node node.exe 2>/dev/null)"; then
  print_tool_version node "$node_bin"
else
  echo "web-dist-ci: node=missing; npm may still provide diagnostics" >&2
fi

cd "$ROOT_DIR"
"$npm_bin" --prefix apps/web ci --ignore-scripts
"$npm_bin" --prefix apps/web run build

DEVE_TRUNK_BIN="$trunk_bin" scripts/smoke-web-release-build.sh

echo "web-dist-ci: ok"
