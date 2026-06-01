#!/usr/bin/env bash
set -euo pipefail

# Fast local gate for ordinary implementation batches. Keep this focused:
# no Docker, no browser, no native package build, and no full workspace test.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export GIT_CONFIG_COUNT="${GIT_CONFIG_COUNT:-1}"
export GIT_CONFIG_KEY_0="${GIT_CONFIG_KEY_0:-safe.directory}"
export GIT_CONFIG_VALUE_0="${GIT_CONFIG_VALUE_0:-$ROOT_DIR}"
TOOL_SHIM_DIR="${DEVE_GATE_TOOL_SHIM_DIR:-${TMPDIR:-/tmp}/deve-gate-tools-${UID:-user}/bin}"

fail() {
  echo "local-quick-gate: $*" >&2
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
  echo "local-quick-gate: run: $*"
  (cd "$ROOT_DIR" && "$@")
}

run git diff --check
run cargo check -p deve_core
run cargo check -p deve_cli
run scripts/plan-coverage.sh --check-no-adr-plan-ref
run scripts/check-acceptance-bindings.sh
run scripts/check-feature-operation-paths.sh

if [[ "${DEVE_QUICK_GATE_TESTS:-1}" == "1" ]]; then
  run cargo test -p deve_core projection_locator_ --lib -- --nocapture
  run cargo test -p deve_cli source_control -- --nocapture
else
  echo "local-quick-gate: skipped focused tests because DEVE_QUICK_GATE_TESTS=${DEVE_QUICK_GATE_TESTS:-}"
fi

echo "local-quick-gate: ok"
