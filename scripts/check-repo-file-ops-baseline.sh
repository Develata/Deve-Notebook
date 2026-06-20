#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The repo-file-ops baseline spec lives in tools/baseline/src/specs/repo_file_ops.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- repo-file-ops
run_deve_baseline "$ROOT_DIR" "repo-file-ops" "repo-file-ops-baseline"
