#!/usr/bin/env bash
set -euo pipefail

# REL-003 dependency audit gate. Local runs may skip unavailable audit tools
# with a diagnostic; CI/release can set DEVE_RELEASE_AUDIT_REQUIRED=1 to make
# missing tools fail closed. First public-tag jobs additionally set
# DEVE_RELEASE_TAG_READY_REQUIRED=1 so registered tag blockers fail closed.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/baseline-wrapper.sh"

AUDIT_REPORT=""

cleanup_current_audit_report() {
  if [[ -n "${AUDIT_REPORT:-}" ]]; then
    rm -f "$AUDIT_REPORT"
    AUDIT_REPORT=""
  fi
}

cleanup_report() {
  local report="$1"
  rm -f "$report"
  if [[ "${AUDIT_REPORT:-}" == "$report" ]]; then
    AUDIT_REPORT=""
  fi
}

trap cleanup_current_audit_report EXIT
trap 'cleanup_current_audit_report; exit 130' INT
trap 'cleanup_current_audit_report; exit 143' TERM

run_cargo_audit() {
  if cargo audit --version >/dev/null 2>&1; then
    local report_rel report
    report_rel="target/release-audit-$RANDOM-$$.json"
    report="$ROOT_DIR/$report_rel"
    AUDIT_REPORT="$report"
    mkdir -p "$ROOT_DIR/target"
    if cargo audit --json >"$report"; then
      if run_deve_baseline "$ROOT_DIR" "release-audit-gate" "release-audit-gate" "cargo-audit-report" "$report_rel"; then
        cleanup_report "$report"
        return
      else
        local status=$?
        cleanup_report "$report"
        return "$status"
      fi
    fi
    cat "$report" >&2 || true
    cleanup_report "$report"
    return 1
  fi

  local cargo_audit_bin
  if cargo_audit_bin="$(baseline_resolve_tool cargo-audit cargo-audit.exe 2>/dev/null)"; then
    local report_rel report
    report_rel="target/release-audit-$RANDOM-$$.json"
    report="$ROOT_DIR/$report_rel"
    AUDIT_REPORT="$report"
    mkdir -p "$ROOT_DIR/target"
    if "$cargo_audit_bin" audit --json >"$report"; then
      if run_deve_baseline "$ROOT_DIR" "release-audit-gate" "release-audit-gate" "cargo-audit-report" "$report_rel"; then
        cleanup_report "$report"
        return
      else
        local status=$?
        cleanup_report "$report"
        return "$status"
      fi
    fi
    cat "$report" >&2 || true
    cleanup_report "$report"
    return 1
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
