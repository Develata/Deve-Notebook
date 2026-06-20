#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The ui-desktop baseline spec lives in tools/baseline/src/specs/ui_desktop.tsv.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- ui-desktop
run_deve_baseline "$ROOT_DIR" "ui-desktop" "ui-desktop-baseline-check"
