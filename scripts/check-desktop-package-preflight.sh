#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

# Delegates to: cargo run -p deve_baseline -- desktop-package-preflight
run_deve_baseline "$ROOT_DIR" "desktop-package-preflight" "desktop-package-preflight-check"
