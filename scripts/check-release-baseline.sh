#!/usr/bin/env bash
set -euo pipefail

# REL-005/REL-006/REL-007/REL-008 keep Docker, compose, release workflow, and
# runtime visibility and smoke surfaces aligned with the current release baseline.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "release-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet -- "$text" "$ROOT_DIR/$file" \
    || fail "missing '$text' in $file"
}

not_contains() {
  local file="$1"
  local text="$2"
  if rg --fixed-strings --quiet -- "$text" "$ROOT_DIR/$file"; then
    fail "unexpected '$text' in $file"
  fi
}

line_no() {
  local file="$1"
  local text="$2"
  rg --line-number --fixed-strings -- "$text" "$ROOT_DIR/$file" | head -n1 | cut -d: -f1
}

assert_before() {
  local file="$1"
  local before="$2"
  local after="$3"
  local before_line
  local after_line
  before_line="$(line_no "$file" "$before")"
  after_line="$(line_no "$file" "$after")"
  [[ -n "$before_line" ]] || fail "missing '$before' in $file"
  [[ -n "$after_line" ]] || fail "missing '$after' in $file"
  (( before_line < after_line )) || fail "'$before' must appear before '$after' in $file"
}

contains ".github/workflows/release.yml" "tags: ['v*']"
contains ".github/workflows/release.yml" "cargo clippy --locked --all-targets -- -D warnings"
contains ".github/workflows/release.yml" "actions/setup-node@v4"
contains ".github/workflows/release.yml" "node-version: 20"
contains ".github/workflows/release.yml" "cargo check --locked -p deve_web --target wasm32-unknown-unknown"
contains ".github/workflows/release.yml" "cargo install cargo-audit --locked"
contains ".github/workflows/release.yml" "DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh"
contains ".github/workflows/release.yml" "scripts/plan-coverage.sh --write-report"
contains ".github/workflows/release.yml" "scripts/check-architecture-registry.sh"
contains ".github/workflows/release.yml" "scripts/check-native-track-boundary.sh"
contains ".github/workflows/release.yml" "scripts/check-native-packaging-gate.sh"
contains ".github/workflows/release.yml" "scripts/check-mobile-platform-package-preflight.sh"
contains ".github/workflows/release.yml" "scripts/check-graph-baseline.sh"
contains ".github/workflows/release.yml" "cargo test --locked"
contains ".github/workflows/release.yml" "docker/build-push-action@v6"
contains ".github/workflows/release.yml" "platforms: linux/amd64"
contains ".github/workflows/release.yml" "type=raw,value=latest"

contains "Dockerfile" "cargo install trunk --locked --version 0.21.14"
contains "Dockerfile" "rustup target add wasm32-unknown-unknown"
contains "Dockerfile" "npm ci --ignore-scripts && npm run build"
contains "Dockerfile" "sed -i 's/\\\\nplugin = false//g' recipe.json"
contains "Dockerfile" "cargo chef cook --release --locked --recipe-path recipe.json"
contains "Dockerfile" "NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build --release"
contains "Dockerfile" "COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/"
contains "Dockerfile" "cargo build --release --locked --package deve_cli"
contains "Dockerfile" "ENV DEVE_LEDGER_DIR=/data/ledger"
contains "Dockerfile" "ENV DEVE_VAULT_PATH=/data/vault"
contains "Dockerfile" "ENV DEVE_BIND_ADDR=0.0.0.0:3001"
contains "Dockerfile" 'CMD ["deve_cli", "serve", "--port", "3001"]'
assert_before "Dockerfile" "sed -i 's/\\\\nplugin = false//g' recipe.json" "cargo chef cook --release --locked --recipe-path recipe.json"
assert_before "Dockerfile" "trunk build --release" "cargo build --release --locked --package deve_cli"
assert_before "Dockerfile" "COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/" "cargo build --release --locked --package deve_cli"

contains "docker-compose.yml" "AUTH_SECRET: \${AUTH_SECRET:?set AUTH_SECRET to at least 32 random bytes}"
contains "docker-compose.yml" "AUTH_PASS: \${AUTH_PASS:?set AUTH_PASS to an Argon2 PHC password hash}"
contains "docker-compose.yml" "image: ghcr.io/develata/deve-notebook:latest"
contains "docker-compose.yml" "container_name: deve-server"
contains "docker-compose.yml" "restart: always"
contains "docker-compose.yml" "- ./data:/data"
contains "docker-compose.yml" "DEVE_BIND_ADDR: 0.0.0.0:3001"
contains "docker-compose.yml" "DEVE_LEDGER_DIR: /data/ledger"
contains "docker-compose.yml" "DEVE_VAULT_PATH: /data/vault"
contains "docker-compose.yml" "mem_limit: 512m"
contains "docker-compose.yml" "http://localhost:3001/api/node/role"
not_contains "docker-compose.yml" "build:"
contains "docker-compose.dev.yml" "build:"
contains "docker-compose.dev.yml" "dockerfile: Dockerfile"
contains "docker-compose.dev.yml" "container_name: deve-server-dev"
contains "docker-compose.dev.yml" "restart: unless-stopped"
contains "docker-compose.dev.yml" "deve-dev-data:/data"

contains ".env.example" "AUTH_SECRET=replace-with-at-least-32-random-bytes"
contains ".env.example" "AUTH_PASS=\$argon2id\$v=19\$m=65536,t=3,p=1\$..."
contains "docs/plan/15_release.md" "runtime image 只交付单个嵌入前端静态资源的 \`deve_cli\` 二进制"
contains "docs/plan/15_release.md" "delivery shape、environment、ports 与聚合 repo"
contains "docs/plan/15_release.md" "health counts。degraded repo"
contains "docs/plan/15_release.md" "native/graph boundary scripts"
contains "docs/features/15_release.md" "Docker/Server 当前主通道是单个 \`deve_cli\` 二进制"
contains "docs/dev-runbook.md" "DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh"
contains "docs/dev-runbook.md" "DEVE_DOCKER_BIN=/path/to/docker"
contains "docs/dev-runbook.md" "scripts/check-native-track-boundary.sh"
contains "docs/dev-runbook.md" "scripts/check-native-packaging-gate.sh"
contains "docs/dev-runbook.md" "scripts/check-mobile-platform-package-preflight.sh"
contains "docs/dev-runbook.md" "scripts/check-graph-baseline.sh"
contains "docs/acceptance-cases/11_commands_settings.md" "scripts/smoke-web-release-build.sh"
not_contains "docs/features/12_commands.md" "NO_COLOR=true trunk build --release"
not_contains "docs/acceptance-cases/11_commands_settings.md" "NO_COLOR=true trunk build --release"
contains "scripts/smoke-docker-release.sh" "DEVE_DOCKER_SMOKE_REQUIRED"
contains "scripts/smoke-docker-release.sh" "DEVE_DOCKER_BIN"
contains "scripts/smoke-docker-release.sh" "docker_cmd build -t"
contains "scripts/smoke-docker-release.sh" "docker_cmd run -d"
contains "scripts/smoke-docker-release.sh" "DEVE_DOCKER_SMOKE_DATA_VOLUME"
contains "scripts/smoke-docker-release.sh" "docker_cmd volume create \"\$DATA_VOLUME\""
contains "scripts/smoke-docker-release.sh" "docker_cmd volume rm -f \"\$DATA_VOLUME\""
contains "scripts/smoke-docker-release.sh" "http://127.0.0.1:\${HOST_PORT}/api/node/role"
contains "scripts/smoke-docker-release.sh" "DEVE_DOCKER_SMOKE_AUTH_PASSWORD"
contains "scripts/smoke-docker-release.sh" "http://127.0.0.1:\${HOST_PORT}/api/auth/login"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-005"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-001"
contains "docs/acceptance-cases/12_tech_release.md" "tags: ['v*']"
contains "docs/acceptance-cases/12_tech_release.md" "type=semver,pattern={{version}}"
contains "docs/acceptance-cases/12_tech_release.md" "type=raw,value=latest"
contains "docs/acceptance-cases/12_tech_release.md" 'ghcr.io/${{ github.repository }}'
not_contains "docs/acceptance-cases/12_tech_release.md" "run: ls dist"
not_contains "docs/acceptance-cases/12_tech_release.md" 'stdout_contains: "v1.0.0"'
contains "docs/acceptance-cases/12_tech_release.md" "DEVE_RELEASE_AUDIT_REQUIRED=1 scripts/check-release-audit-gate.sh"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-006"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-007"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-008"
contains "docs/acceptance-cases/12_tech_release.md" "DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh"
contains "docs/acceptance-cases/12_tech_release.md" "DEVE_DOCKER_BIN=/path/to/docker"
contains "docs/acceptance-cases/12_tech_release.md" "DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/smoke-runtime-happy-path.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/smoke-runtime-recovery-path.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-release-baseline.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-native-track-boundary.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-native-packaging-gate.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-desktop-package-preflight.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-desktop-platform-package-build.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-mobile-platform-package-preflight.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-graph-baseline.sh"
contains "scripts/check-mobile-platform-package-preflight.sh" "DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED"
contains "scripts/check-mobile-platform-package-preflight.sh" "DEVE_MOBILE_PACKAGE_TARGETS"
contains "scripts/check-mobile-platform-package-preflight.sh" "cargo tauri android --help"
contains "scripts/check-mobile-platform-package-preflight.sh" "cargo tauri ios --help"
contains "scripts/check-mobile-platform-package-preflight.sh" "package build remains closed in this gate"
contains "docs/acceptance-cases/12_tech_release.md" 'json_fields_present: ["version", "profile", "delivery", "environment"]'
contains "docs/acceptance-cases/12_tech_release.md" 'json_fields_present: ["repo_health.status", "repo_health.local_total", "repo_health.degraded"]'
contains "docs/features/15_release.md" "版本、profile、环境、交付形态和 repo health 聚合状态"
contains "docs/dev-runbook.md" "repo_health.status=degraded"
contains "apps/cli/src/server/node_role_http.rs" '"version": r.version'
contains "apps/cli/src/server/node_role_http.rs" '"profile": r.profile'
contains "apps/cli/src/server/node_role_http.rs" '"delivery": r.delivery'
contains "apps/cli/src/server/node_role_http.rs" '"environment": r.environment'
contains "apps/cli/src/server/node_role_http.rs" '"repo_health":'
contains "apps/cli/src/server/node_role.rs" "from_degraded_count_clamps_degraded_count_to_local_total"
contains "apps/cli/src/server/start.rs" "repo_health_summary"
contains "apps/cli/src/server/static_files.rs" '"embedded-frontend"'
contains "apps/web/src/api/connection_role.rs" "format_node_role_summary"
contains "apps/web/src/api/connection_role.rs" "format_repo_health"
contains "apps/web/src/components/dashboard/runtime_card.rs" "RuntimeCard"
contains "apps/web/src/components/settings.rs" 'env!("CARGO_PKG_VERSION")'
contains "apps/web/src/main.rs" "Web app mount skipped"
not_contains "apps/web/src/main.rs" "web_sys::window().unwrap()"
not_contains "apps/web/src/main.rs" "window.document().unwrap()"
contains "scripts/smoke-runtime-release-info.sh" "/api/node/role"
contains "scripts/smoke-runtime-release-info.sh" "DEVE_RUNTIME_SMOKE_REQUIRED"
contains "scripts/smoke-runtime-release-info.sh" "allowed_delivery"
contains "scripts/smoke-runtime-release-info.sh" "repo_health"
contains "scripts/smoke-runtime-release-info.sh" "repo_health counts do not add up"
contains "scripts/smoke-runtime-release-info.sh" "unknown repo_health must use zero counts"
contains "scripts/smoke-runtime-happy-path.sh" "ws_endpoint_sync_hello_uses_switched_repo_scope"
contains "scripts/smoke-runtime-happy-path.sh" "ws_endpoint_register_writer_after_sync_hello_returns_write_ready"
contains "scripts/smoke-runtime-happy-path.sh" "ws_edit_after_register_writer_emits_new_op_and_ack"
contains "scripts/smoke-runtime-happy-path.sh" "ws_open_doc_and_history_read_back_registered_edit"
contains "scripts/smoke-runtime-happy-path.sh" "restore_runs_only_on_clean_reconnect_edge"
contains "scripts/smoke-runtime-happy-path.sh" "expected exactly one executed test"
contains "scripts/smoke-runtime-happy-path.sh" "expected passing test"
contains "scripts/smoke-runtime-recovery-path.sh" "degraded_local"
contains "scripts/smoke-runtime-recovery-path.sh" "sync_scope_cleanup"
contains "scripts/smoke-runtime-recovery-path.sh" "write_gate"
contains "scripts/smoke-runtime-recovery-path.sh" "message_refresh"
contains "scripts/smoke-runtime-recovery-path.sh" "status_summary"
contains "scripts/smoke-runtime-recovery-path.sh" "auth_probe"
contains "scripts/smoke-runtime-recovery-path.sh" "expected at least one test"
contains "scripts/check-release-audit-gate.sh" "cargo audit"
contains "scripts/check-release-audit-gate.sh" "npm --prefix"
contains "scripts/check-release-audit-gate.sh" "DEVE_RELEASE_AUDIT_REQUIRED"
contains "scripts/check-release-audit-gate.sh" "skip cargo audit"
contains "docs/features/operations/release_quality_gates.md" "runtime happy-path and recovery smokes remain explicit local/CI script gates"

echo "release-baseline-check: ok"
