#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "native-packaging-gate-check: $*" >&2
  exit 1
}

check_no_tauri_lock_entries() {
  if [[ -f "$ROOT_DIR/Cargo.lock" ]] \
    && rg -n 'name = "(tauri|tauri-build)"' "$ROOT_DIR/Cargo.lock" >/dev/null; then
    fail "real Tauri crates must not appear in Cargo.lock before the packaging gate opens"
  fi
}

"$ROOT_DIR/scripts/check-native-track-boundary.sh"
check_no_tauri_lock_entries

cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture
cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture

check_no_tauri_lock_entries

echo "native-packaging-gate-check: ok"
