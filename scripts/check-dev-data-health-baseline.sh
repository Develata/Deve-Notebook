#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The dev-data-health baseline spec lives in tools/baseline/src/specs/dev_data_health.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- dev-data-health
run_deve_baseline "$ROOT_DIR" "dev-data-health" "check-dev-data-health-baseline"
