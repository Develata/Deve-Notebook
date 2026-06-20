#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The auth baseline spec lives in tools/baseline/src/specs/auth.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- auth
run_deve_baseline "$ROOT_DIR" "auth" "auth-baseline-check"
