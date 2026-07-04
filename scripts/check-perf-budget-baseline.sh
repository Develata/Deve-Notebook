#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for the PERF-001 lightweight baseline.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "perf-budget" "perf-budget-baseline-check"
