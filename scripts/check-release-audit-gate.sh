#!/usr/bin/env bash
set -euo pipefail

# REL-003 dependency audit gate. Local runs may skip unavailable audit tools
# with a diagnostic; CI/release can set DEVE_RELEASE_AUDIT_REQUIRED=1 to make
# missing tools fail closed.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

run_cargo_audit() {
  if cargo audit --version >/dev/null 2>&1; then
    cargo audit
    return
  fi

  run_deve_baseline "$ROOT_DIR" "release-audit-gate" "release-audit-gate" "cargo-audit-missing"
}

run_npm_audit() {
  [[ -f "$ROOT_DIR/apps/web/package-lock.json" ]] || return

  if command -v npm >/dev/null 2>&1; then
    npm --prefix "$ROOT_DIR/apps/web" audit --audit-level=high
    return
  fi

  run_deve_baseline "$ROOT_DIR" "release-audit-gate" "release-audit-gate" "npm-audit-missing"
}

run_deve_baseline "$ROOT_DIR" "release-audit-gate" "release-audit-gate"
run_cargo_audit
run_npm_audit

echo "release-audit-gate: ok"
