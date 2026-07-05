#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for the REL-013 reliability/observability governance baseline.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "reliability-observability" "reliability-observability-baseline-check"
