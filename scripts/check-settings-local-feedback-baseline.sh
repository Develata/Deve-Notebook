#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The settings-local-feedback baseline spec lives in tools/baseline/src/specs/settings_local_feedback.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- settings-local-feedback
run_deve_baseline "$ROOT_DIR" "settings-local-feedback" "settings-local-feedback-baseline-check"
