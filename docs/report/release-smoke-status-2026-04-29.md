# Release Smoke Status 2026-04-29

This report records the Docker release/runtime smoke status after the Docker
Desktop WSL integration became available again.

## Summary

- Overall status: Docker release smoke is now passing.
- Code gate status: previously passing; no new code gate regression observed in
  this Docker batch.
- Docker release smoke status: image build passed, container started, and the
  host endpoint probe returned 200.
- Latest successful command:

```bash
DEVE_DOCKER_SMOKE_REQUIRED=1 \
DEVE_DOCKER_SMOKE_PORT=3102 \
scripts/smoke-docker-release.sh
```

Observed result:

```text
docker-release-smoke: ok
```

## Docker Environment

`docker info` now works from the same WSL shell used by Codex.

Observed Docker context:

- Client version: `29.4.1`
- Server version: `29.4.1`
- Context: `default`
- Server OS: Docker Desktop Linux engine
- Kernel: WSL2 Linux kernel

This closes the previous host blocker where plain Linux `docker info` failed
inside the agent shell.

## Endpoint Smoke

The smoke script built `deve-notebook:local-smoke`, started the release image
with production auth material, and verified:

```text
http://127.0.0.1:3102/api/node/role
```

returned HTTP 200.

During diagnosis, the first rerun showed that the image and application were
healthy even while the host probe failed:

- Container logs showed the server listening on `0.0.0.0:3001`.
- `docker exec <container> curl -fsS http://127.0.0.1:3001/api/node/role`
  returned the node-role JSON.
- Docker healthcheck reported `healthy`.

The host probe failure was caused by local proxy/WSL port behavior. The smoke
script now explicitly bypasses proxy handling for local endpoint probes and
prints container health plus internal endpoint diagnostics if the host endpoint
does not become healthy.

## Script Follow-Up

`scripts/smoke-docker-release.sh` now:

- Supports `DEVE_DOCKER_BIN`.
- Prints Docker binary/context diagnostics when Docker is unavailable.
- Uses `curl --noproxy "127.0.0.1,localhost"` for local endpoint probes.
- Emits container health and internal `/api/node/role` diagnostics before
  failing host endpoint verification.

## Non-Blocking Warnings

The Docker frontend build printed:

```text
Browserslist: caniuse-lite is outdated.
```

This is a tooling freshness warning from the frontend build chain, not a release
smoke failure. It can be handled separately if the frontend dependency baseline
is intentionally refreshed.

## Current Release Follow-Up

- Docker release smoke endpoint verification is closed for the current image.
- Keep `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and this Docker smoke as the release gates before a tag.
- If host endpoint smoke regresses again, first inspect proxy variables and WSL
  port forwarding before treating it as an application runtime failure.
