#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper for existing CI/runbook calls.
# The feature-operation-paths check lives in tools/baseline/src/feature_operation_paths.rs.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_deve_baseline "$ROOT_DIR" "feature-operation-paths" "feature-operation-path-check"
