#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The i18n-formatting baseline spec lives in tools/baseline/src/specs/i18n_formatting.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "i18n-formatting-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- i18n-formatting"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- i18n-formatting
); then
  fail "cargo run -p deve_baseline -- i18n-formatting failed"
fi
