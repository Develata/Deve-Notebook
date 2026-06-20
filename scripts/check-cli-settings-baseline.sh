#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The cli-settings baseline spec lives in tools/baseline/src/specs/cli_settings.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- cli-settings
run_deve_baseline "$ROOT_DIR" "cli-settings" "cli-settings-baseline-check"
