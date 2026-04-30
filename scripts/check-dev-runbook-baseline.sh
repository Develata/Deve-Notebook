#!/usr/bin/env bash
set -euo pipefail

# REL-004 keeps the current runtime runbook aligned with implemented startup,
# auth, frontend, Chrome MCP, search, and verification boundaries.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNBOOK="$ROOT_DIR/docs/dev-runbook.md"
ACCEPTANCE="$ROOT_DIR/docs/acceptance-cases/12_tech_release.md"

fail() {
  echo "check-dev-runbook-baseline: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet "$text" "$file" || fail "missing '$text' in $file"
}

contains "$RUNBOOK" 'cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001'
contains "$RUNBOOK" 'cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001'
contains "$RUNBOOK" 'NO_COLOR=true trunk build --release'
contains "$RUNBOOK" 'NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080'
contains "$RUNBOOK" 'AUTH_SECRET'
contains "$RUNBOOK" 'AUTH_PASS'
contains "$RUNBOOK" 'admin` /'
contains "$RUNBOOK" 'chrome-mcp http://127.0.0.1:8080/'
contains "$RUNBOOK" 'scripts/check-auth-baseline.sh'
contains "$RUNBOOK" 'scripts/check-network-baseline.sh'
contains "$RUNBOOK" 'scripts/check-cli-settings-baseline.sh'
contains "$RUNBOOK" 'scripts/check-browser-prefs-boundary.sh'
contains "$RUNBOOK" 'scripts/check-search-baseline.sh'
contains "$RUNBOOK" 'scripts/check-rendering-baseline.sh'
contains "$RUNBOOK" 'scripts/check-ai-baseline.sh'
contains "$RUNBOOK" 'scripts/check-source-control-baseline.sh'
contains "$RUNBOOK" 'scripts/check-dev-runbook-baseline.sh'
contains "$RUNBOOK" 'scripts/check-ws-structured-errors.sh'
contains "$RUNBOOK" 'scripts/check-release-baseline.sh'
contains "$RUNBOOK" 'scripts/check-native-track-boundary.sh'
contains "$RUNBOOK" 'scripts/check-graph-baseline.sh'
contains "$RUNBOOK" 'scripts/check-architecture-registry.sh'
contains "$RUNBOOK" 'scripts/plan-coverage.sh'
contains "$RUNBOOK" 'DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh'
contains "$RUNBOOK" 'scripts/smoke-runtime-release-info.sh'
contains "$RUNBOOK" 'repo_health.status=degraded'
contains "$RUNBOOK" '/api/admin/projection-check'
contains "$RUNBOOK" 'DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh'
contains "$RUNBOOK" 'scripts/smoke-docker-release.sh'

contains "$ACCEPTANCE" 'case_id: REL-004'
contains "$ACCEPTANCE" 'scripts/check-dev-runbook-baseline.sh'
