#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- native-track-boundary
run_deve_baseline "$ROOT_DIR" "native-track-boundary" "native-track-boundary-check"
