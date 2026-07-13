#!/usr/bin/env bash
set -euo pipefail

# Fast local gate for ordinary implementation batches. Keep this focused:
# no Docker, no browser, no native package build, and no full workspace test.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export GIT_CONFIG_COUNT="${GIT_CONFIG_COUNT:-1}"
export GIT_CONFIG_KEY_0="${GIT_CONFIG_KEY_0:-safe.directory}"
export GIT_CONFIG_VALUE_0="${GIT_CONFIG_VALUE_0:-$ROOT_DIR}"
TOOL_SHIM_DIR="${DEVE_GATE_TOOL_SHIM_DIR:-${TMPDIR:-/tmp}/deve-gate-tools-${UID:-user}/bin}"
LOCAL_CARGO_TARGET_DIR="${DEVE_LOCAL_QUICK_GATE_TARGET_DIR:-target/local-quick-gate}"

source "$ROOT_DIR/scripts/baseline-wrapper.sh"
run_deve_baseline "$ROOT_DIR" "local-quick-gate" "local-quick-gate"

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

cargo_bin="$(baseline_resolve_tool cargo cargo.exe || true)"
if [[ -z "$cargo_bin" ]]; then
  echo "local-quick-gate: missing required tool 'cargo'" >&2
  exit 1
fi
install_tool_shim cargo "$cargo_bin"
export PATH="$TOOL_SHIM_DIR:$PATH"

run() {
  echo "local-quick-gate: run: $*"
  (cd "$ROOT_DIR" && "$@")
}

run git diff --check
run cargo check --target-dir "$LOCAL_CARGO_TARGET_DIR" -p deve_core
run cargo check --target-dir "$LOCAL_CARGO_TARGET_DIR" -p deve_cli
run scripts/plan-coverage.sh --check-no-adr-plan-ref
run scripts/check-acceptance-matrix.sh
run scripts/check-feature-operation-paths.sh

if [[ "${DEVE_QUICK_GATE_TESTS:-1}" == "1" ]]; then
  run cargo test --target-dir "$LOCAL_CARGO_TARGET_DIR" -p deve_core projection_locator_ --lib -- --nocapture
  run cargo test --target-dir "$LOCAL_CARGO_TARGET_DIR" -p deve_cli source_control_http_scope_requires_nonzero_nonce -- --nocapture
  run cargo test --target-dir "$LOCAL_CARGO_TARGET_DIR" -p deve_cli source_control_scope_nonce_gate_rejects_missing_scope_before_handler -- --nocapture
else
  echo "local-quick-gate: skipped focused tests because DEVE_QUICK_GATE_TESTS=${DEVE_QUICK_GATE_TESTS:-}"
fi

echo "local-quick-gate: ok"
