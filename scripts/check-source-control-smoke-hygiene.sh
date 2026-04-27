#!/usr/bin/env bash
set -euo pipefail

# DIFF-010 keeps Source Control smoke tests from assuming Git cleanliness equals
# Deve Source Control cleanliness.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "check-source-control-smoke-hygiene: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet "$text" "$file" || fail "missing '$text' in $file"
}

RUNBOOK="$ROOT_DIR/docs/dev-runbook.md"
MAIN="$ROOT_DIR/apps/cli/src/main.rs"
DISPATCH="$ROOT_DIR/apps/cli/src/dispatch.rs"
COMMAND="$ROOT_DIR/apps/cli/src/commands/sc_status.rs"
FIXTURE_TEST="$ROOT_DIR/apps/cli/src/server/source_control_http_clean_fixture_test.rs"
ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/04_diff.md"

contains "$RUNBOOK" 'sc-status --repo <repo>'
contains "$RUNBOOK" 'Source Control state lives in Deve'
contains "$RUNBOOK" 'checked-in local `default` ledger being clean'

contains "$MAIN" 'ScStatus'
contains "$DISPATCH" 'commands::sc_status::run'
contains "$COMMAND" 'list_staged_in_local_repo'
contains "$COMMAND" 'list_pending_fs_in_local_repo'
contains "$COMMAND" 'sc_status['
contains "$FIXTURE_TEST" 'clean_source_control_smoke_fixture_stage_unstage_commit'
contains "$FIXTURE_TEST" 'ProxyHarness::spawn'
contains "$FIXTURE_TEST" '/api/sc/stage-pending'
contains "$FIXTURE_TEST" '/api/sc/unstage'
contains "$FIXTURE_TEST" '/api/sc/commit'

contains "$ACCEPTANCE" 'case_id: DIFF-010'
contains "$ACCEPTANCE" 'scripts/check-source-control-smoke-hygiene.sh'
contains "$ACCEPTANCE" 'clean_source_control_smoke_fixture'
