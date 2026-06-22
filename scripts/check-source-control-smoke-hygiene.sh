#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The source-control-smoke-hygiene spec lives in tools/baseline/src/specs/source_control_smoke_hygiene.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "source-control-smoke-hygiene" "check-source-control-smoke-hygiene"
