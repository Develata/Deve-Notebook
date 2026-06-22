#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The browser-prefs-boundary spec lives in tools/baseline/src/specs/browser_prefs_boundary.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "browser-prefs-boundary" "browser-prefs-boundary-check"
