#!/usr/bin/env bash
set -euo pipefail

# REL-003 dependency audit gate. Local runs may skip unavailable audit tools
# with a diagnostic; CI/release can set DEVE_RELEASE_AUDIT_REQUIRED=1 to make
# missing tools fail closed.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "release-audit-gate: $*" >&2
  exit 1
}

is_required() {
  [[ "${DEVE_RELEASE_AUDIT_REQUIRED:-0}" == "1" || "${1:-0}" == "1" ]]
}

run_cargo_audit() {
  if cargo audit --version >/dev/null 2>&1; then
    cargo audit
    return
  fi

  local msg="cargo-audit unavailable; install with 'cargo install cargo-audit --locked' or set DEVE_CARGO_AUDIT_REQUIRED=0 for local diagnostic-only runs"
  if is_required "${DEVE_CARGO_AUDIT_REQUIRED:-0}"; then
    fail "$msg"
  fi
  echo "release-audit-gate: skip cargo audit: $msg" >&2
}

run_npm_audit() {
  [[ -f "$ROOT_DIR/apps/web/package-lock.json" ]] || return

  if command -v npm >/dev/null 2>&1; then
    npm --prefix "$ROOT_DIR/apps/web" audit --audit-level=high
    return
  fi

  local msg="npm unavailable; install Node.js/npm or set DEVE_NPM_AUDIT_REQUIRED=0 for local diagnostic-only runs"
  if is_required "${DEVE_NPM_AUDIT_REQUIRED:-0}"; then
    fail "$msg"
  fi
  echo "release-audit-gate: skip npm audit: $msg" >&2
}

run_cargo_audit
run_npm_audit

echo "release-audit-gate: ok"
