#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- native-packaging-gate
run_deve_baseline "$ROOT_DIR" "native-packaging-gate" "native-packaging-gate-check"
