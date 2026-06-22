#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The ws-structured-errors spec lives in tools/baseline/src/specs/ws_structured_errors.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "ws-structured-errors" "ws-structured-errors-check"
