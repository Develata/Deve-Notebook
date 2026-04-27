#!/usr/bin/env bash
set -euo pipefail

# REL-005 keeps Docker, compose, and release workflow surfaces aligned with
# the current single-binary embedded-frontend release baseline.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "release-baseline-check: $*" >&2
  exit 1
}

contains() {
  local file="$1"
  local text="$2"
  rg --fixed-strings --quiet "$text" "$ROOT_DIR/$file" \
    || fail "missing '$text' in $file"
}

line_no() {
  local file="$1"
  local text="$2"
  rg --line-number --fixed-strings "$text" "$ROOT_DIR/$file" | head -n1 | cut -d: -f1
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
contains ".github/workflows/release.yml" "cargo check --locked -p deve_web --target wasm32-unknown-unknown"
contains ".github/workflows/release.yml" "scripts/plan-coverage.sh --write-report"
contains ".github/workflows/release.yml" "scripts/check-architecture-registry.sh"
contains ".github/workflows/release.yml" "cargo test --locked"
contains ".github/workflows/release.yml" "docker/build-push-action@v6"
contains ".github/workflows/release.yml" "platforms: linux/amd64"
contains ".github/workflows/release.yml" "type=raw,value=latest"

contains "Dockerfile" "cargo install trunk --locked --version 0.21.14"
contains "Dockerfile" "rustup target add wasm32-unknown-unknown"
contains "Dockerfile" "npm ci --ignore-scripts && npm run build"
contains "Dockerfile" "trunk build --release"
contains "Dockerfile" "COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/"
contains "Dockerfile" "cargo build --release --locked --package deve_cli"
contains "Dockerfile" "ENV DEVE_LEDGER_DIR=/data/ledger"
contains "Dockerfile" "ENV DEVE_VAULT_PATH=/data/vault"
contains "Dockerfile" "ENV DEVE_BIND_ADDR=0.0.0.0:3001"
contains "Dockerfile" 'CMD ["deve_cli", "serve", "--port", "3001"]'
assert_before "Dockerfile" "trunk build --release" "cargo build --release --locked --package deve_cli"
assert_before "Dockerfile" "COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/" "cargo build --release --locked --package deve_cli"

contains "docker-compose.yml" "AUTH_SECRET: \${AUTH_SECRET:?set AUTH_SECRET to at least 32 random bytes}"
contains "docker-compose.yml" "AUTH_PASS: \${AUTH_PASS:?set AUTH_PASS to an Argon2 PHC password hash}"
contains "docker-compose.yml" "DEVE_BIND_ADDR: 0.0.0.0:3001"
contains "docker-compose.yml" "DEVE_LEDGER_DIR: /data/ledger"
contains "docker-compose.yml" "DEVE_VAULT_PATH: /data/vault"
contains "docker-compose.yml" "mem_limit: 512m"
contains "docker-compose.yml" "http://localhost:3001/api/node/role"

contains ".env.example" "AUTH_SECRET=replace-with-at-least-32-random-bytes"
contains ".env.example" 'AUTH_PASS=$argon2id$v=19$m=65536,t=3,p=1$...'
contains "docs/plan/15_release.md" 'runtime image ships a single `deve_cli` binary with embedded frontend static assets'
contains "docs/features/15_release.md" 'Docker/Server 当前主通道是单个 `deve_cli` 二进制'
contains "docs/dev-runbook.md" "DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh"
contains "scripts/smoke-docker-release.sh" "DEVE_DOCKER_SMOKE_REQUIRED"
contains "scripts/smoke-docker-release.sh" "docker build -t"
contains "scripts/smoke-docker-release.sh" "docker run -d"
contains "scripts/smoke-docker-release.sh" "http://127.0.0.1:\${HOST_PORT}/api/node/role"
contains "docs/acceptance-cases/12_tech_release.md" "case_id: REL-005"
contains "docs/acceptance-cases/12_tech_release.md" "DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh"
contains "docs/acceptance-cases/12_tech_release.md" "scripts/check-release-baseline.sh"

echo "release-baseline-check: ok"
