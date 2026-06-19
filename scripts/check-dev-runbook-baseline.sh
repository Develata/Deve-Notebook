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
  MSYS2_ARG_CONV_EXCL="$text" rg --fixed-strings --quiet "$text" "$file" || fail "missing '$text' in $file"
}

contains "$RUNBOOK" 'cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001'
contains "$RUNBOOK" 'cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001'
contains "$RUNBOOK" 'scripts/smoke-web-release-build.sh'
contains "$RUNBOOK" 'scripts/smoke-web-runtime-paths.sh'
contains "$RUNBOOK" "The wrapper normalizes Trunk's \`NO_COLOR\` parsing"
contains "$RUNBOOK" 'NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080'
contains "$RUNBOOK" 'AUTH_SECRET'
contains "$RUNBOOK" 'AUTH_PASS'
contains "$RUNBOOK" 'admin` /'
contains "$RUNBOOK" 'chrome-mcp http://127.0.0.1:8080/'
while IFS= read -r guard_script; do
  contains "$RUNBOOK" "$guard_script"
done < <(cd "$ROOT_DIR" && find scripts -maxdepth 1 -type f -name 'check-*.sh' | LC_ALL=C sort)

contains "$RUNBOOK" 'scripts/plan-coverage.sh'
contains "$RUNBOOK" 'scripts/smoke-web-release-build.sh'
contains "$RUNBOOK" 'DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh'
contains "$RUNBOOK" 'scripts/smoke-runtime-release-info.sh'
contains "$RUNBOOK" 'scripts/smoke-runtime-happy-path.sh'
contains "$RUNBOOK" 'repo switch, `SyncHello`, `RegisterWriter`, document create, edit ack'
contains "$RUNBOOK" 'scripts/smoke-runtime-recovery-path.sh'
contains "$RUNBOOK" 'degraded local projection write gates'
contains "$RUNBOOK" 'repo_health.status=degraded'
contains "$RUNBOOK" '/api/admin/projection-check'
contains "$RUNBOOK" 'DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh'
contains "$RUNBOOK" 'scripts/smoke-docker-release.sh'
contains "$RUNBOOK" 'DEVE_DOCKER_MULTI_REQUIRED=1 bash scripts/smoke-docker-multiclient.sh'
contains "$RUNBOOK" 'scripts/smoke-docker-multiclient.mjs'
contains "$RUNBOOK" 'DEVE_DOCKER_MULTI_KEEP=1'
contains "$RUNBOOK" 'DEVE_DOCKER_P2P_MESH_REQUIRED=1 bash scripts/smoke-docker-p2p-mesh.sh'
contains "$RUNBOOK" 'scripts/smoke-docker-p2p-mesh.sh'
contains "$RUNBOOK" 'docker-compose.mesh.yml'
contains "$RUNBOOK" 'DEVE_DOCKER_P2P_MESH_KEEP=1'
contains "$RUNBOOK" 'docker-compose.yml` is the production compose entry'
contains "$RUNBOOK" 'ghcr.io/develata/deve-notebook:latest'
contains "$RUNBOOK" 'docker compose -f docker-compose.dev.yml up --build'
contains "$RUNBOOK" 'Native Shell Modes'
contains "$RUNBOOK" 'LocalBackend'
contains "$RUNBOOK" 'RemoteBrowser'
contains "$RUNBOOK" 'DEVE_NATIVE_REMOTE_URL=https://example.invalid'

contains "$ACCEPTANCE" 'case_id: REL-004'
contains "$ACCEPTANCE" 'case_id: REL-009'
contains "$ACCEPTANCE" 'case_id: REL-010'
contains "$ACCEPTANCE" 'case_id: REL-011'
contains "$ACCEPTANCE" 'scripts/check-dev-runbook-baseline.sh'
