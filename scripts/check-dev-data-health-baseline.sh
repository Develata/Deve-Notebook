#!/usr/bin/env bash
set -euo pipefail

# CMD-009 keeps projection health diagnostics operator-facing and fail-closed.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "check-dev-data-health-baseline: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet "$text" "$file" || fail "missing '$text' in $file"
}

RUNBOOK="$ROOT_DIR/docs/dev-runbook.md"
DIAG="$ROOT_DIR/crates/core/src/sync/projection_diagnostic.rs"
ADMIN_API="$ROOT_DIR/apps/cli/src/admin_api.rs"
NODE_CHECK="$ROOT_DIR/apps/cli/src/commands/node_check.rs"
ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/11_commands_settings.md"

contains "$RUNBOOK" 'node-check --projection --repo <repo>'
contains "$RUNBOOK" 'repair --check --repo <repo>'
contains "$RUNBOOK" 'status=authority_corrupt'
contains "$RUNBOOK" 'rebuild_supported=false'
contains "$RUNBOOK" 'issue_code=missing_parent'
contains "$RUNBOOK" 'repair_hint'
contains "$RUNBOOK" 'repair-step preflight'
contains "$RUNBOOK" 'byte-for-byte immutability'
contains "$RUNBOOK" 'repair --repo <repo> --rebuild-projection'

contains "$DIAG" 'repair_hint'
contains "$DIAG" 'projection rebuild is unsupported'
contains "$ADMIN_API" 'pub repair_hint: String'
contains "$NODE_CHECK" 'repair_hint:'
contains "$NODE_CHECK" 'SyncManager::new_checked'
contains "$ROOT_DIR/apps/cli/src/commands/repair/mod.rs" 'repair-check'

contains "$ACCEPTANCE" 'scripts/check-dev-data-health-baseline.sh'
contains "$ACCEPTANCE" 'deve repair --check --help'
contains "$ACCEPTANCE" 'cargo test -p deve_cli repair_check -- --nocapture'
contains "$ACCEPTANCE" 'cargo test -p deve_cli node_check -- --nocapture'
