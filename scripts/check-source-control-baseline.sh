#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The source-control baseline spec lives in tools/baseline/src/specs/source_control.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- source-control
run_deve_baseline "$ROOT_DIR" "source-control" "source-control-baseline-check"
