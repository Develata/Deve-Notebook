# Full Workspace Verification Pass 2026-04-30

The P2 full workspace verification pass is complete.

## Result

- Blocking failures: 0 after fixes in this batch.
- `scripts/check-ws-structured-errors.sh` initially failed on
  `apps/web/src/api/git_mirror.rs::last_error`. This was a guard false positive:
  `last_error` is a Git mirror repair-review HTTP diagnostic compatibility
  field, not a WebSocket protocol `error: String` shape. The guard now matches
  the exact `error` field name.
- `cargo clippy --all-targets --all-features -- -D warnings` initially failed
  on a needless borrow in `apps/cli/src/commands/serve.rs`. The bind preflight
  now passes `bind_addr` directly.
- Docker release smoke passed, but reproduced the Docker-only cargo-chef
  skeleton warning family:
  `unused manifest key: ... plugin`. Current checked-in manifests still do not
  contain `plugin = ...`; this remains a Docker/cargo-chef recipe-skeleton
  cleanup item, not a product runtime blocker.

## Verified

```bash
scripts/check-auth-baseline.sh
scripts/check-network-baseline.sh
scripts/check-cli-settings-baseline.sh
scripts/check-browser-prefs-boundary.sh
scripts/check-search-baseline.sh
scripts/check-rendering-baseline.sh
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
scripts/smoke-web-release-build.sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
DEVE_RUNTIME_BASE_URL=http://127.0.0.1:3101 DEVE_RUNTIME_SMOKE_REQUIRED=1 scripts/smoke-runtime-release-info.sh
DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh
```

Runtime smoke used isolated data roots under `/tmp/deve-runtime-smoke-*`.

## Remaining Follow-Up

Closed by `cargo-chef-skeleton-warning-cleanup-2026-04-30.md`. The Dockerfile
now strips only cargo-chef generated `\nplugin = false` recipe noise before
`cargo chef cook`, and Docker release smoke completed with
`docker-release-smoke: ok`.
