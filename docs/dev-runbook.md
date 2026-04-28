# Current Runtime Runbook

This runbook describes the current implemented startup and test paths. It is not
a roadmap for future desktop/mobile native apps, server-backed Settings API, or
full Tantivy indexing.

## Local Backend

Use explicit development mode for local runs:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

`--dev` sets `DEVE_ENV=development` for the current serve process when the
environment variable is unset. The default development login is `admin` /
`admin`. These defaults are only valid for `--dev` or explicit
`DEVE_ENV=development`.

To include the current lightweight search runtime gate:

```bash
cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001
```

Without the `search` feature, search requests must fail closed with a structured
unavailable error.

## Local Frontend

Preferred embedded path:

```bash
cd apps/web
NO_COLOR=true trunk build --release
cd ../..
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

Open `http://127.0.0.1:3001/`. The CLI embeds `apps/web/dist` at build time, so
after Web source changes you must rebuild `apps/web/dist` before rebuilding or
running the CLI. Otherwise the embedded server can serve stale WASM.

Fallback two-process path:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
```

```bash
cd apps/web
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080/`. This path avoids embedded-asset staleness during
UI work. Backend-only `serve --dev` may return 404 on `/` when neither embedded
assets nor a valid `DEVE_STATIC_DIR` are available; API and WS routes are still
the backend runtime boundary.

## Production Auth

Production is the default when `--dev` is not used and `DEVE_ENV` is not
`development`. Production must provide:

- `AUTH_SECRET`: JWT signing secret, at least 32 bytes.
- `AUTH_PASS`: Argon2 PHC password hash.
- `AUTH_USER`: optional username, defaults to `admin`.

If `AUTH_SECRET` or `AUTH_PASS` is missing, startup must exit non-zero with the
production auth error. For local testing, use `--dev` rather than weakening this
production boundary.

## Projection Health

Startup can skip a local repo when its Structure Facts authority cannot build a
safe tree projection. This is a repo-local degraded state, not a global server
startup failure.

Use read-only diagnostics first:

```bash
cargo run -p deve_cli --bin deve_cli -- node-check --projection --repo <repo>
```

Important fields:

- `status=healthy`: projection authority can be used.
- `status=authority_corrupt`: Structure Facts are corrupt.
- `rebuild_supported=false`: `repair --rebuild-projection` must not rewrite this
  authority automatically.
- `issue_code=missing_parent`: a node references a parent that is absent from
  Structure Facts.
- `repair_hint`: operator-facing next step for the diagnostic class.

For rebuild-supported projection drift, use:

```bash
cargo run -p deve_cli --bin deve_cli -- repair --repo <repo> --rebuild-projection
```

For `authority_corrupt` repos, inspect ledger/backups and restore authoritative
Structure Facts before expecting scan, watcher, export, or source-control paths
to treat that repo as healthy. The server should continue serving other healthy
repos.

## Source Control Smoke Hygiene

Source Control state lives in Deve's ledger/staging tables, not in Git's working
tree. `git status` can be clean while the app still has staged or unstaged
Source Control entries.

Before running browser commit/stage smoke tests, inspect the target repo:

```bash
cargo run -p deve_cli --bin deve_cli -- sc-status --repo <repo>
```

The command is read-only. It prints separate `staged` and `unstaged` counts so a
dirty dev fixture is visible before the UI test starts. Do not make acceptance
cases depend on the checked-in local `default` ledger being clean; use a seeded
temporary repo for exact counts, or assert only that Source Control loads and
reports its current state.

## Docker Release Smoke

After enabling Docker Desktop WSL integration or another Docker-compatible
runtime, run:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh
```

The script builds the local Dockerfile, starts the image with production
`AUTH_SECRET` / `AUTH_PASS` material, waits for
`http://127.0.0.1:3001/api/node/role`, then removes the smoke container and
temporary data directory. Without `DEVE_DOCKER_SMOKE_REQUIRED=1`, a machine that
does not provide Docker reports a skip instead of failing the local baseline.

## Runtime Release Info Smoke

With a local backend already running, validate the public runtime/release shape
endpoint:

```bash
DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh
```

The script checks `role`, `version`, `profile`, `delivery`, `environment`,
`ws_port`, `main_port`, and the aggregated `repo_health` object from
`/api/node/role`. Use
`DEVE_RUNTIME_BASE_URL=http://127.0.0.1:<port>` when testing a non-default port.
Without `DEVE_RUNTIME_SMOKE_REQUIRED=1`, an unavailable local server reports a
skip instead of failing the baseline.

`repo_health.status=degraded` means at least one local repo was skipped for
projection execution while the server stayed available for other healthy repos.
The public endpoint intentionally exposes aggregate counts only. Use
`node-check --projection --repo <repo>` or protected `/api/admin/projection-check`
for repo-specific details.

## Chrome MCP Smoke

In WSL2, if Chrome MCP cannot connect because `127.0.0.1:9222` is down, run:

```bash
chrome-mcp http://127.0.0.1:8080/
```

Use `http://127.0.0.1:3001/` for the embedded path or
`http://127.0.0.1:8080/` for the Trunk fallback path. For search smoke testing,
start the backend with `--features search`, log in with the development account,
and verify the UI reaches `Ready` before submitting a search query.

## Verification

Targeted tests are preferred while implementing:

```bash
cargo test -p deve_cli <filter> -- --nocapture
cargo test -p deve_core <filter> -- --nocapture
cargo test -p deve_web <filter> -- --nocapture
```

Current docs/code guard scripts:

```bash
scripts/check-auth-baseline.sh
scripts/check-network-baseline.sh
scripts/check-cli-settings-baseline.sh
scripts/check-search-baseline.sh
scripts/check-ai-baseline.sh
scripts/check-source-control-baseline.sh
scripts/check-source-control-smoke-hygiene.sh
scripts/check-dev-data-health-baseline.sh
scripts/check-native-track-boundary.sh
scripts/check-graph-baseline.sh
scripts/check-dev-runbook-baseline.sh
scripts/check-ws-structured-errors.sh
scripts/check-release-baseline.sh
scripts/check-architecture-registry.sh
scripts/plan-coverage.sh
scripts/smoke-runtime-release-info.sh
scripts/smoke-docker-release.sh
```

Use full-suite checks as release/final verification, not as the default inner
loop on a low-memory machine:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
