#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint for existing CI/runbook calls.
# The storage/repo baseline spec lives in tools/baseline/src/specs/storage_repo.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "storage-repo-baseline-check: $*" >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required to run cargo run -p deve_baseline -- storage-repo"
fi

if ! (
  cd "$ROOT_DIR"
  cargo run -p deve_baseline -- storage-repo
); then
  fail "cargo run -p deve_baseline -- storage-repo failed"
fi
