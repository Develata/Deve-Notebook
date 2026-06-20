#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The ui-token baseline spec lives in tools/baseline/src/specs/ui_token.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-token-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- ui-token"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- ui-token
); then
  fail "cargo run -p deve_baseline -- ui-token failed"
fi
