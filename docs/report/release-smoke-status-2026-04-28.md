# Release Smoke Status 2026-04-28

This report records the current release/runtime smoke status after the P2
Runtime / Release / UI Debt pass. It is historical evidence only; the
authoritative contracts remain `docs/plan/15_release.md`,
`docs/acceptance-cases/12_tech_release.md`, `.github/workflows/release.yml`, and
the current code.

## Summary

- Overall status: release quality gates are green except Docker smoke, which is
  blocked by the host Docker daemon being unreachable.
- Code gate status: passing.
- Runtime smoke status: passing for local embedded frontend development mode.
- Docker release smoke status: environment-blocked, not a code failure.
- Worktree note: `.codex` remained untracked and was not part of release smoke
  validation.

## Passing Gates

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --check` | Passed |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| Web WASM check | `cargo check --locked -p deve_web --target wasm32-unknown-unknown` | Passed |
| Full locked tests | `cargo test --locked` | Passed |
| Plan coverage | `scripts/plan-coverage.sh --summary-missing-plan-ref` | Passed with 0 blocking violations |
| Release baseline | `scripts/check-release-baseline.sh` | Passed |
| Architecture registry | `scripts/check-architecture-registry.sh` | Passed |
| Native boundary | `scripts/check-native-track-boundary.sh` | Passed |
| Graph baseline | `scripts/check-graph-baseline.sh` | Passed |

## Runtime Smoke

Local server command:

```bash
cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3101
```

Runtime endpoint smoke:

```bash
DEVE_RUNTIME_SMOKE_REQUIRED=1 \
DEVE_RUNTIME_BASE_URL=http://127.0.0.1:3101 \
scripts/smoke-runtime-release-info.sh
```

Observed result:

```text
runtime-release-info-smoke: ok: main v0.0.1 standard embedded-frontend development repos:degraded(1/2)
```

Direct `/api/node/role` response shape included:

```json
{
  "delivery": "embedded-frontend",
  "environment": "development",
  "main_port": 3101,
  "profile": "standard",
  "repo_health": {
    "degraded": 1,
    "healthy": 1,
    "local_total": 2,
    "status": "degraded"
  },
  "role": "main",
  "version": "0.0.1",
  "ws_port": 3101
}
```

Chrome MCP dashboard smoke:

- Opened `http://127.0.0.1:3101/`.
- Logged in with development credentials `admin` / `admin`.
- Dashboard rendered the embedded frontend and displayed:
  `main (ws:3101) | v0.0.1 | standard | embedded-frontend | development | repos:degraded (1/2)`.

This validates the REL-006 runtime shape, including degraded repo aggregate
visibility without leaking repo-specific corruption details through the public
endpoint.

## Docker Smoke

Attempted command:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 \
DEVE_DOCKER_SMOKE_PORT=3102 \
scripts/smoke-docker-release.sh
```

Result in both sandboxed and escalated contexts:

```text
docker-release-smoke: docker daemon is not reachable
```

Assessment: REL-002 remains blocked by host Docker availability. The smoke script
itself already validates prerequisites, uses a temporary container/data
directory, and cleans them up on exit. No code failure was observed because the
Docker build/run phase never started.

## Remaining Release Follow-Up

- Re-run `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=3102 scripts/smoke-docker-release.sh`
  after Docker Desktop WSL integration or another Docker-compatible daemon is
  available.
- Keep `cargo test --locked` as the final full-suite gate before any release tag.
- Treat native Tauri packaging, OS signing, GitHub Release binary uploads, and
  multi-arch Docker as deferred delivery work until `docs/plan/15_release.md`
  explicitly promotes them into the current baseline.
