#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "ui-spa-routing-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local pattern="$2"
  rg -q --fixed-strings -- "$pattern" "$ROOT_DIR/$file" \
    || fail "missing '$pattern' in $file"
}

# UI-WEB-001: non-runtime Web routes fall back to SPA index; API/WS do not.
contains docs/acceptance-cases/05_ui.md "case_id: UI-WEB-001"
contains docs/acceptance-cases/05_ui.md "scripts/check-ui-spa-routing-baseline.sh"
contains docs/acceptance-cases/05_ui.md "cargo test -p deve_cli static_files -- --nocapture"
contains docs/acceptance-cases/05_ui.md "spa_route_fallback_status_200 true"
contains docs/acceptance-cases/05_ui.md "api_route_not_spa_fallback true"
contains docs/plan/08_ui_design_01_web.md "Serve(path) \\to index.html"

contains apps/cli/src/server/static_files.rs "is_spa_fallback_path"
contains apps/cli/src/server/static_files.rs "ServeDir::new(&dir).fallback(fallback)"
contains apps/cli/src/server/static_files/tests.rs "static_dir_spa_route_returns_index_with_ok_status"
contains apps/cli/src/server/static_files/tests.rs "static_dir_unknown_api_route_does_not_fallback_to_index"
contains apps/cli/src/server/static_files/tests.rs "static_dir_unknown_ws_route_does_not_fallback_to_index"
contains apps/cli/src/server/static_files/tests.rs "static_dir_serves_existing_asset_without_spa_fallback"
contains apps/cli/src/server/static_files_embed.rs "asset_for_request_path_in"
contains apps/cli/src/server/static_files_embed.rs "embedded_lookup_rejects_api_route_before_spa_fallback"
contains apps/cli/src/server/static_files_embed.rs "embedded_lookup_rejects_ws_route_before_spa_fallback"

echo "ui-spa-routing-baseline-check: ok"
