#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

NODE_BIN="$(baseline_resolve_tool node node.exe)" \
  || { echo "android-document-create-contract-check: node is required" >&2; exit 1; }
flow_test="$ROOT_DIR/scripts/android-document-create-flow.test.mjs"
settlement_test="$ROOT_DIR/scripts/android-document-create-settlement.test.mjs"
if baseline_repo_on_wsl_windows_mount "$ROOT_DIR" && baseline_windows_exe "$NODE_BIN"; then
  flow_test="$(wslpath -w "$flow_test")"
  settlement_test="$(wslpath -w "$settlement_test")"
fi

"$NODE_BIN" --test "$flow_test" "$settlement_test"

echo "android-document-create-contract-check: ok"
