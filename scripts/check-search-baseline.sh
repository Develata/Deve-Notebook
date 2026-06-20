#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The search baseline spec lives in tools/baseline/src/specs/search.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "search-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- search"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- search
); then
  fail "cargo run -p deve_baseline -- search failed"
fi
