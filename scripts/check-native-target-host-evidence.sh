#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-target-host-evidence-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  if command -v rg >/dev/null 2>&1; then
    rg --fixed-strings --quiet -- "$text" "$file" \
      || fail "missing '$text' in ${file#$ROOT_DIR/}"
  else
    grep -F -- "$text" "$file" >/dev/null \
      || fail "missing '$text' in ${file#$ROOT_DIR/}"
  fi
}

has_line() {
  local file="$1"
  local text="$2"
  if command -v rg >/dev/null 2>&1; then
    rg --fixed-strings --line-regexp --quiet -- "$text" "$file"
  else
    grep -Fx -- "$text" "$file" >/dev/null
  fi
}

validate_report() {
  local file="$1"

  [[ -f "$file" ]] || fail "missing evidence file: ${file#$ROOT_DIR/}"

  contains "$file" "# Native Target-host Evidence"
  contains "$file" "Target:"
  contains "$file" "Workflow run:"
  contains "$file" "Host OS:"
  contains "$file" "Tool versions:"
  contains "$file" "Commands:"
  contains "$file" "Command results:"
  contains "$file" "Artifact paths:"
  contains "$file" "Install result:"
  contains "$file" "Startup result:"
  contains "$file" "Process runtime gate: closed"
  contains "$file" "Native authority writes: closed"
  contains "$file" "Conclusion:"

  if has_line "$file" "Target: Desktop macOS" \
    || has_line "$file" "Target: Desktop Windows"; then
    contains "$file" "desktop_preflight="
    contains "$file" "process_gate="
    contains "$file" "invalid_startup_request="
    contains "$file" "invalid_installer_request="
    contains "$file" "package_build="
    contains "$file" "startup_smoke="
    contains "$file" "native_session_smoke="
    contains "$file" "installer_smoke="
  fi
}

if (($# == 0)); then
  set -- "$ROOT_DIR/docs/report/native-target-host-evidence-template.md"
fi

for report in "$@"; do
  if [[ "$report" = /* ]]; then
    validate_report "$report"
  else
    validate_report "$ROOT_DIR/$report"
  fi
done

echo "native-target-host-evidence-check: ok"
