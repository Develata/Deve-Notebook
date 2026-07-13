#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- acceptance-matrix
run_deve_baseline "$ROOT_DIR" "acceptance-matrix" "acceptance-matrix-check"
