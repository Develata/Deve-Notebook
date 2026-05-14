#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_TARGET:-Local diagnostic}"
SLUG="$(
  printf '%s' "$TARGET" \
    | tr '[:upper:]' '[:lower:]' \
    | sed 's/[^a-z0-9][^a-z0-9]*/-/g; s/^-//; s/-$//'
)"
OUT="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_OUT:-$ROOT_DIR/target/native-target-host-evidence/${SLUG:-local}.md}"
WORKFLOW_RUN="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_WORKFLOW_RUN:-}"
COMMANDS="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_COMMANDS:-scripts/write-native-target-host-evidence.sh}"
ARTIFACTS="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS:-N/A - package artifact not produced by this diagnostic run}"
INSTALL_RESULT="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_INSTALL_RESULT:-N/A - install/startup smoke must be captured by target-host package execution evidence}"
STARTUP_RESULT="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_STARTUP_RESULT:-N/A - install/startup smoke must be captured by target-host package execution evidence}"
CONCLUSION="${DEVE_NATIVE_TARGET_HOST_EVIDENCE_CONCLUSION:-diagnostic-only}"

if [[ -z "$WORKFLOW_RUN" ]]; then
  if [[ -n "${GITHUB_SERVER_URL:-}" && -n "${GITHUB_REPOSITORY:-}" && -n "${GITHUB_RUN_ID:-}" ]]; then
    WORKFLOW_RUN="$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"
  else
    WORKFLOW_RUN="N/A - local target-host execution"
  fi
fi

tool_version() {
  local label="$1"
  shift
  local executable="$1"
  local output
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf -- '- %s: unavailable\n' "$label"
    return
  fi
  output="$("$@" 2>&1 | head -n1 || true)"
  if [[ -n "$output" ]]; then
    printf -- '- %s: %s\n' "$label" "$output"
  else
    printf -- '- %s: unavailable\n' "$label"
  fi
}

host_os() {
  local kernel
  kernel="$(uname -a 2>/dev/null || printf 'unknown')"
  if command -v sw_vers >/dev/null 2>&1; then
    printf '%s; %s\n' "$kernel" "$(sw_vers 2>/dev/null | tr '\n' '; ' | sed 's/[; ]*$//')"
  elif [[ "${OS:-}" == "Windows_NT" ]]; then
    printf '%s; Windows_NT\n' "$kernel"
  else
    printf '%s\n' "$kernel"
  fi
}

mkdir -p "$(dirname "$OUT")"

{
  printf '# Native Target-host Evidence - %s - %s\n\n' "$TARGET" "$(date +%F)"
  printf 'Target: %s\n\n' "$TARGET"
  printf 'Workflow run: %s\n\n' "$WORKFLOW_RUN"
  printf 'Host OS: %s\n\n' "$(host_os)"
  printf 'Tool versions:\n\n'
  tool_version rustc rustc --version
  tool_version "cargo tauri" cargo tauri --version
  tool_version node node --version
  tool_version npm npm --version
  tool_version xcodebuild xcodebuild -version
  tool_version "MSVC cl.exe" cl.exe
  tool_version "WiX Toolset" wix --version
  tool_version "NSIS makensis" makensis -VERSION
  printf '\nCommands:\n\n'
  printf '```bash\n%s\n```\n\n' "$COMMANDS"
  printf 'Artifact paths:\n\n%s\n\n' "$ARTIFACTS"
  printf 'Install result: %s\n\n' "$INSTALL_RESULT"
  printf 'Startup result: %s\n\n' "$STARTUP_RESULT"
  printf 'Process runtime gate: closed\n\n'
  printf 'Native authority writes: closed\n\n'
  printf 'Conclusion: %s\n' "$CONCLUSION"
} >"$OUT"

"$ROOT_DIR/scripts/check-native-target-host-evidence.sh" "$OUT"

echo "native-target-host-evidence-write: $OUT"
