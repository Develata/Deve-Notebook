#!/usr/bin/env bash
set -euo pipefail

# CMD-007A Web release build smoke.
# Normalizes environment quirks observed in WSL/Trunk:
# - Trunk 0.21 expects NO_COLOR to be a bool-like value, not "1".
# - Trunk's Tailwind pipeline may emit non-actionable Browserslist DB freshness
#   noise even though this repo does not lock browserslist/caniuse-lite.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRUNK_BIN="${DEVE_TRUNK_BIN:-trunk}"

export NO_COLOR="${NO_COLOR:-true}"
if [[ "$NO_COLOR" == "1" ]]; then
  export NO_COLOR=true
fi
export BROWSERSLIST_IGNORE_OLD_DATA="${BROWSERSLIST_IGNORE_OLD_DATA:-true}"

cd "$ROOT_DIR/apps/web"
echo "web-release-build-smoke: trunk=$TRUNK_BIN"
"$TRUNK_BIN" build --release
