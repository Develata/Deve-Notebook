#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- native-process-adapter-gate
run_deve_baseline "$ROOT_DIR" "native-process-adapter-gate" "native-process-adapter-gate-check"
