#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The i18n-hardcoded baseline spec lives in tools/baseline/src/specs/i18n_hardcoded.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "i18n-hardcoded" "i18n-hardcoded-baseline-check"
