#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The diff-color baseline spec lives in tools/baseline/src/specs/diff_color.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "diff-color-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- diff-color"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- diff-color
); then
  fail "cargo run -p deve_baseline -- diff-color failed"
fi
