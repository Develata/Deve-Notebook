#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-dashboard-refresh-baseline-check: $*" >&2
  exit 1
}

check_contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

check_absent() {
  local file="$1"
  local pattern="$2"
  if rg -q --fixed-strings "$pattern" "$ROOT_DIR/$file"; then
    fail "forbidden '$pattern' in $file"
  fi
}

# UI-WEB-003: Dashboard health refreshes from WS SystemMetrics and remains RAM-only.
check_contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-003"
check_contains docs/acceptance-cases/05_ui.md "run: scripts/check-ui-dashboard-refresh-baseline.sh"
check_contains docs/acceptance-cases/05_ui.md "run: cargo test -p deve_web dashboard_metrics -- --nocapture"
check_contains docs/acceptance-cases/05_ui.md "ui_assert: system_health_refreshed true"
check_contains docs/acceptance-cases/05_ui.md "ui_assert: sync_status_updates_via_ws true"

check_contains apps/cli/src/server/metrics.rs "std::time::Duration::from_secs(5)"
check_contains apps/cli/src/server/start.rs "metrics::spawn_broadcaster(app_state.clone());"
check_contains crates/core/src/protocol/server.rs "SystemMetrics"

check_contains apps/web/src/hooks/use_core/dashboard_context.rs "pub sample_seq: u64"
check_contains apps/web/src/hooks/use_core/dashboard_context.rs "pub metrics_live: ReadSignal<bool>"
check_contains apps/web/src/api/service.rs "pub connection_epoch: ReadSignal<u64>"
check_contains apps/web/src/api/service/tests.rs "fn dashboard_metrics_stale_connection_epoch_is_not_current()"
check_contains apps/web/src/api/incoming.rs "push_server_message(queue, next_seq, connection_epoch, server_msg);"
check_contains apps/web/src/api/connection.rs "set_connection_epoch.set(connection_epoch);"
check_contains apps/web/src/hooks/use_core/effects/message.rs "is_current_connection_message(connection_epoch, current_connection_epoch)"
check_contains apps/web/src/hooks/use_core/effects/message.rs "fn dashboard_metrics_stale_connection_epoch_is_skipped_by_message_effect()"
check_contains apps/web/src/hooks/use_core/state_init/runtime/sync.rs "let (system_metrics_live, set_system_metrics_live) = signal(false);"
check_contains apps/web/src/hooks/use_core/mod.rs "reset_dashboard_metrics_live_on_disconnect(ws.status, signals.set_system_metrics_live);"
check_contains apps/web/src/hooks/use_core/mod.rs "fn dashboard_metrics_live_resets_on_disconnect_states()"
check_contains apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "metrics.sample_seq.saturating_add(1)"
check_contains apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "signals.set_system_metrics_live.set(true);"
check_contains apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "fn dashboard_metrics_ws_refresh_increments_sample_seq()"
check_contains apps/web/src/components/dashboard/mod.rs "data-deve-dashboard-metrics-state"
check_contains apps/web/src/components/dashboard/mod.rs "ctx.metrics_live.get()"
check_contains apps/web/src/components/dashboard/mod.rs "fn dashboard_metrics_state_tracks_ws_refresh_and_disconnect_freeze()"
check_contains apps/web/src/components/dashboard/health_card.rs "data-deve-dashboard-health-source=\"ws-system-metrics\""
check_contains apps/web/src/components/dashboard/health_card.rs "data-deve-dashboard-health-sample"
check_contains apps/web/src/components/dashboard/sync_card.rs "data-deve-dashboard-sync-source=\"ws-system-metrics\""
check_contains apps/web/src/components/dashboard/sync_card.rs "data-deve-dashboard-sync-sample"

check_absent apps/web/src/hooks/use_core/dashboard_context.rs "localStorage"
check_absent apps/web/src/hooks/use_core/dashboard_context.rs "indexedDB"
check_absent apps/web/src/hooks/use_core/dashboard_context.rs "local_storage"
check_absent apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "localStorage"
check_absent apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "indexedDB"
check_absent apps/web/src/hooks/use_core/effects/message_runtime_remaining.rs "local_storage"
check_absent apps/web/src/components/dashboard/mod.rs "localStorage"
check_absent apps/web/src/components/dashboard/mod.rs "indexedDB"
check_absent apps/web/src/components/dashboard/mod.rs "local_storage"

echo "ui-dashboard-refresh-baseline-check: ok"
