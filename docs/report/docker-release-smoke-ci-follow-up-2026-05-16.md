# Docker Release Smoke CI Follow-up

Date: 2026-05-16

## Scope

- Close the Docker release smoke blocker left by the full regression gate.
- Keep the production Docker path on the same `scripts/smoke-docker-release.sh` contract used locally.
- Do not change `docs/plan/`.

## Findings

- Local Docker remains a host issue: WSL `docker` returns `SIGBUS`, and Windows Docker Desktop reports a Linux engine `500 Internal Server Error`.
- GitHub-hosted Docker runner is healthy and is the correct validation host for this batch.
- First CI run `25963144187` failed in the old `cargo-chef` layer because the generated recipe path drifted from the current lock/build graph.
- Second CI run `25963330037` passed frontend build but failed in backend `cargo build --locked` because `Cargo.lock` existed locally but was ignored and not tracked in Git.

## Changes

- Added manual `Docker Smoke` workflow for isolated Docker validation without publishing GHCR images.
- Removed the Dockerfile `cargo-chef` build path and kept a direct locked release build.
- Moved Dockerfile NodeSource setup to Node 24.
- Tracked `Cargo.lock` and guarded release baseline against future lockfile ignore/regression.

## Verification

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `cargo metadata --locked --format-version 1 --no-deps`
- `cargo check --release --locked --package deve_cli`
- `git diff --check`
- GitHub Actions `Docker Smoke` run `25963571993`: success, 9m46s, `HEAD f3f23e1e`.

## Result

Docker release smoke is closed on a healthy CI Docker host. The remaining local Docker failure is host-environment only, not an application or Dockerfile blocker.
