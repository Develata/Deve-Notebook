#!/usr/bin/env bash
set -euo pipefail

check_contains() {
  local file="$1"
  local pattern="$2"
  if ! rg -q --fixed-strings "$pattern" "$file"; then
    echo "auth-unauthorized-check: missing '$pattern' in $file" >&2
    exit 1
  fi
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$file"; then
    echo "auth-unauthorized-check: forbidden '$pattern' in $file" >&2
    exit 1
  fi
}

check_contains apps/web/src/api/auth_probe.rs "matches!(status, 401 | 403) || has_auth_error_code"
check_contains apps/web/src/api/connection.rs ".try_set(signals.set_status, ConnectionStatus::Unauthorized)"
check_contains apps/web/src/api/connection.rs ".try_set(signals.set_status, ConnectionStatus::Disconnected)"
check_contains apps/web/src/components/main_layout/setup.rs "ws_status.get() == ConnectionStatus::Unauthorized"
check_contains apps/web/src/components/disconnect_overlay.rs "ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None"
check_contains apps/web/src/hooks/use_core/effects/message_protocol/control.rs "let Some(switch_nonce) = switch_nonce else"
check_absent apps/web/src/hooks/use_core/effects/message_protocol/control.rs "expect(\"checked above\")"
check_absent apps/web/src/api/output.rs "set_status.set(ConnectionStatus::Disconnected);"

echo "auth-unauthorized-check: ok"
