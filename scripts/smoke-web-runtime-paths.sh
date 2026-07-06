#!/usr/bin/env bash
set -euo pipefail

# plan_ref:
#   - 14_commands#command-palette-shortcuts

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cat <<'MSG'
web-runtime-paths-smoke:

Fresh smoke data root prep:
  export DEVE_WEB_RUNTIME_SMOKE_ROOT="${DEVE_WEB_RUNTIME_SMOKE_ROOT:-target/codex-smoke/web-runtime}"
  mkdir -p "$DEVE_WEB_RUNTIME_SMOKE_ROOT/projection-base"
  export DEVE_LEDGER_DIR="$DEVE_WEB_RUNTIME_SMOKE_ROOT/config-root/ledger"
  cargo run -p deve_cli --bin deve_cli -- init --path "$DEVE_WEB_RUNTIME_SMOKE_ROOT/config-root" --repo default --projection-base "$DEVE_WEB_RUNTIME_SMOKE_ROOT/projection-base"

CMD-007A embedded browser path:
  scripts/smoke-web-release-build.sh
  cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
  open http://127.0.0.1:3001/

CMD-007B Trunk fallback path:
  cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
  cd apps/web
  NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
  open http://127.0.0.1:8080/
MSG

if [[ "${DEVE_WEB_RUNTIME_SMOKE_BUILD:-0}" == "1" ]]; then
  "$ROOT_DIR/scripts/smoke-web-release-build.sh"
fi
