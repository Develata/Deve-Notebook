#!/usr/bin/env bash
set -euo pipefail

# Explicit deep audit for release prep or broad architecture changes. This is
# intentionally slower than the local quick gate and includes governance,
# baseline scripts, runtime smokes, and optional full cargo tests.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export GIT_CONFIG_COUNT="${GIT_CONFIG_COUNT:-1}"
export GIT_CONFIG_KEY_0="${GIT_CONFIG_KEY_0:-safe.directory}"
export GIT_CONFIG_VALUE_0="${GIT_CONFIG_VALUE_0:-$ROOT_DIR}"
TOOL_SHIM_DIR="${DEVE_GATE_TOOL_SHIM_DIR:-${TMPDIR:-/tmp}/deve-gate-tools-${UID:-user}/bin}"

source "$ROOT_DIR/scripts/baseline-wrapper.sh"
run_deve_baseline "$ROOT_DIR" "deep-audit-gate" "deep-audit-gate"

fail() {
  echo "deep-audit-gate: $*" >&2
  exit 1
}

windows_path_to_unix() {
  local path="$1"
  local drive rest lower
  if [[ "$path" =~ ^([A-Za-z]):\\(.*)$ ]]; then
    drive="${BASH_REMATCH[1]}"
    rest="${BASH_REMATCH[2]//\\//}"
    lower="$(printf '%s' "$drive" | tr '[:upper:]' '[:lower:]')"
    printf '/mnt/%s/%s\n' "$lower" "$rest"
    printf '/%s/%s\n' "$lower" "$rest"
  fi
}

is_runnable_tool() {
  local path="$1"
  [[ -n "$path" ]] || return 1
  [[ -f "$path" || -x "$path" ]] || return 1
  "$path" --version >/dev/null 2>&1
}

resolve_tool() {
  local tool_name="$1"
  shift
  local candidate candidate_path converted
  for candidate in "$tool_name" "$@"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      candidate_path="$(command -v "$candidate")"
      if is_runnable_tool "$candidate_path"; then
        printf '%s\n' "$candidate_path"
        return 0
      fi
    fi
    if command -v where.exe >/dev/null 2>&1; then
      candidate_path="$(where.exe "$candidate" 2>/dev/null | tr -d '\r' | head -n1 || true)"
      if is_runnable_tool "$candidate_path"; then
        printf '%s\n' "$candidate_path"
        return 0
      fi
      while IFS= read -r converted; do
        if is_runnable_tool "$converted"; then
          printf '%s\n' "$converted"
          return 0
        fi
      done < <(windows_path_to_unix "$candidate_path")
    fi
  done
  fail "missing required tool '$tool_name'"
}

install_tool_shim() {
  local name="$1"
  local target="$2"
  mkdir -p "$TOOL_SHIM_DIR"
  cat >"$TOOL_SHIM_DIR/$name" <<EOF
#!/usr/bin/env bash
exec "$target" "\$@"
EOF
  chmod +x "$TOOL_SHIM_DIR/$name"
}

cargo_bin="$(resolve_tool cargo cargo.exe)"
install_tool_shim cargo "$cargo_bin"
export PATH="$TOOL_SHIM_DIR:$PATH"

run() {
  echo "deep-audit-gate: run: $*"
  (cd "$ROOT_DIR" && "$@")
}

if [[ "${DEVE_DEEP_AUDIT_WRITE_REPORT:-0}" == "1" ]]; then
  run scripts/plan-coverage.sh --write-report --check-reverse-coverage
else
  run scripts/plan-coverage.sh --check-reverse-coverage
  echo "deep-audit-gate: skipped plan coverage report; set DEVE_DEEP_AUDIT_WRITE_REPORT=1 to write scripts/plan-coverage.txt"
fi
run scripts/plan-coverage.sh --check-metadata-completeness
run scripts/plan-coverage.sh --check-perf-budget
run scripts/plan-coverage.sh --check-no-adr-plan-ref
run scripts/plan-coverage.sh --check-md-links
run scripts/plan-coverage-selftest.sh
run scripts/check-architecture-registry.sh

baseline_scripts=(
  scripts/check-auth-baseline.sh
  scripts/check-auth-unauthorized-state.sh
  scripts/check-network-baseline.sh
  scripts/check-foundation-baseline.sh
  scripts/check-cli-settings-baseline.sh
  scripts/check-settings-local-feedback-baseline.sh
  scripts/check-browser-prefs-boundary.sh
  scripts/check-storage-repo-baseline.sh
  scripts/check-search-baseline.sh
  scripts/check-rendering-baseline.sh
  scripts/check-ai-baseline.sh
  scripts/check-source-control-baseline.sh
  scripts/check-source-control-smoke-hygiene.sh
  scripts/check-repo-file-ops-baseline.sh
  scripts/check-dev-data-health-baseline.sh
  scripts/check-graph-baseline.sh
  scripts/check-diff-color-baseline.sh
  scripts/check-large-doc-baseline.sh
  scripts/check-i18n-hardcoded-baseline.sh
  scripts/check-i18n-formatting-baseline.sh
  scripts/check-mobile-baseline.sh
  scripts/check-ui-dashboard-refresh-baseline.sh
  scripts/check-ui-desktop-baseline.sh
  scripts/check-ui-disconnect-baseline.sh
  scripts/check-ui-focus-baseline.sh
  scripts/check-ui-spa-routing-baseline.sh
  scripts/check-ui-token-baseline.sh
  scripts/check-ui-z-index-baseline.sh
  scripts/check-dev-runbook-baseline.sh
  scripts/check-ws-structured-errors.sh
  scripts/check-release-baseline.sh
  scripts/check-native-track-boundary.sh
  scripts/check-native-packaging-gate.sh
  scripts/check-native-process-adapter-gate.sh
  scripts/check-mobile-platform-package-preflight.sh
  scripts/check-mobile-android-shell-package-build.sh
  scripts/check-desktop-signing-preflight.sh
  scripts/check-mobile-android-release-preflight.sh
)

for script in "${baseline_scripts[@]}"; do
  run "$script"
done

run scripts/check-release-audit-gate.sh
run scripts/smoke-runtime-happy-path.sh
run scripts/smoke-runtime-recovery-path.sh

if [[ "${DEVE_DEEP_AUDIT_FULL_TESTS:-0}" == "1" ]]; then
  run cargo test
else
  echo "deep-audit-gate: skipped full cargo test; set DEVE_DEEP_AUDIT_FULL_TESTS=1 to run it"
fi

if [[ "${DEVE_DEEP_AUDIT_DOCKER_SMOKE:-0}" == "1" ]]; then
  run env DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
else
  echo "deep-audit-gate: skipped Docker smoke; set DEVE_DEEP_AUDIT_DOCKER_SMOKE=1 to run it"
fi

echo "deep-audit-gate: ok"
