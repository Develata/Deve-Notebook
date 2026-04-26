#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ws-structured-errors-check: $*" >&2
  exit 1
}

SCAN_PATHS=(
  "$ROOT_DIR/crates/core/src/protocol"
  "$ROOT_DIR/apps/cli/src/server"
  "$ROOT_DIR/apps/web/src/api"
  "$ROOT_DIR/apps/web/src/hooks/use_core/effects"
  "$ROOT_DIR/apps/web/src/editor/sync"
)

if rg -n 'Error\(String\)' "${SCAN_PATHS[@]}" --glob '*.rs'; then
  fail "legacy Error(String) protocol shape found"
fi

if rg -n 'error:\s*(Option<)?String' "${SCAN_PATHS[@]}" --glob '*.rs'; then
  fail "stringly-typed protocol error field found"
fi

rg -q 'ProtocolError \{ error: ServerError' \
  "$ROOT_DIR/crates/core/src/protocol/server.rs" \
  || fail "ServerMessage::ProtocolError is not backed by ServerError"

rg -q 'PluginResponse \{.*error: Option<ServerError>' \
  "$ROOT_DIR/crates/core/src/protocol/server.rs" \
  || fail "PluginResponse error is not backed by ServerError"

rg -q 'ServerMessage::ProtocolError' \
  "$ROOT_DIR/apps/web/src/hooks/use_core/effects/message_dispatch_route_protocol.rs" \
  || fail "web protocol dispatch no longer handles ProtocolError explicitly"

rg -q 'ServerErrorCode' "$ROOT_DIR/apps/web/src/i18n/server_error.rs" \
  || fail "web server error i18n mapping is missing ServerErrorCode"

echo "ws-structured-errors-check: ok"
