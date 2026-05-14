# Docker Release Smoke Freshness - 2026-05-14

本报告记录当前 Dockerfile 的生产容器 smoke 复验。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Plan basis: `15_release.md`, `REL-002`, `REL-008`.
- Code scope: `scripts/smoke-docker-release.sh`, release baseline guard, dev runbook.
- Non-goal: change Dockerfile layers, release compose shape, production auth contract, or `docs/plan/`.

## Finding

Fresh smoke found a host/runtime-specific storage issue:

- WSL default `docker` command was unavailable in this environment.
- Windows Docker CLI at `/mnt/c/Program Files/Docker/Docker/resources/bin/docker.exe` worked after Docker Desktop daemon startup.
- The Docker image built successfully, but the first container run using a WSL temporary bind mount failed at startup with `Permission denied (os error 13)` while creating `/data/ledger`.
- `chmod 0777` on the WSL temporary directory was insufficient under Docker Desktop bind-mount semantics.

The failure was not a production auth issue or app startup regression. It was a smoke harness storage mounting issue: the runtime image runs as non-root `appuser`, and Docker Desktop did not present the WSL temp bind mount as writable for that user.

## Fixes

- Docker release smoke now stores `/data` in a temporary Docker named volume.
- `DEVE_DOCKER_SMOKE_DATA_VOLUME` may point the smoke at a caller-managed named volume.
- Auto-created smoke data volumes are removed during cleanup; caller-managed volumes are preserved.
- Release baseline now guards the named-volume behavior.
- Dev runbook now states that the smoke removes a temporary Docker data volume.

## Verification

Ran:

- `DEVE_DOCKER_BIN='/mnt/c/Program Files/Docker/Docker/resources/bin/docker.exe' DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash -n scripts/smoke-docker-release.sh scripts/check-release-baseline.sh scripts/check-dev-runbook-baseline.sh`

Results:

- Docker image build: pass.
- Production container startup: pass.
- `/api/node/role`: `200`.
- Production login smoke: `200`.
- Output: `docker-release-smoke: ok`.

## Residual Notes

- Plain `npm audit` still reports one moderate Mermaid advisory during the Docker frontend build. Current release audit gate blocks high and critical advisories; this remains a separate dependency maintenance task.
- WSL users without Docker Desktop WSL integration can run this smoke by setting `DEVE_DOCKER_BIN` to a reachable Docker-compatible CLI.
