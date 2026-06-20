#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The ui-focus baseline spec lives in tools/baseline/src/specs/ui_focus.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-focus-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- ui-focus"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- ui-focus
); then
  fail "cargo run -p deve_baseline -- ui-focus failed"
fi
