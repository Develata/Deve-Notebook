#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The dev-runbook baseline spec lives in tools/baseline/src/specs/dev_runbook.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "check-dev-runbook-baseline: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- dev-runbook"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- dev-runbook
); then
  fail "cargo run -p deve_baseline -- dev-runbook failed"
fi
