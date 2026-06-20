#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The mobile baseline spec lives in tools/baseline/src/specs/mobile.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- mobile
run_deve_baseline "$ROOT_DIR" "mobile" "mobile-baseline-check"
