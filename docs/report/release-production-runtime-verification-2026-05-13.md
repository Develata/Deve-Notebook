# Release / Production Runtime Verification - 2026-05-13

本报告记录 `REL-002/006/007/008` 与生产 auth 边界的 post-queue 验证。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实与本批修复。

## Scope

- `docs/plan/15_release.md`
- `docs/plan/09_auth.md`
- `docs/plan/08_ui_design_01_web.md`
- `docs/acceptance-cases/12_tech_release.md`
- `docs/acceptance-cases/08_auth.md`

## Results

Passed:

- Web release build produced `apps/web/dist` successfully.
- Production startup without `AUTH_SECRET` / `AUTH_PASS` exited non-zero with `Production mode requires AUTH_SECRET and AUTH_PASS`.
- Production startup with explicit `AUTH_SECRET`, `AUTH_USER`, and valid Argon2 PHC `AUTH_PASS` succeeded.
- `/api/node/role` returned:
  - `role = main`
  - `version = 0.0.1`
  - `profile = standard`
  - `delivery = static-dir-override`
  - `environment = production`
  - `repo_health.status = healthy`
  - `repo_health.local_total = 1`
  - `repo_health.degraded = 0`
- `scripts/smoke-runtime-release-info.sh` accepted the running production instance.
- Production login with correct password returned `200 {"success":true}` and set `HttpOnly; SameSite=Strict; Secure; Path=/` cookie.
- Production login with wrong password returned `401 {"success":false,"code":"AUTH_INVALID_PASSWORD"}`.
- Chrome MCP loaded the production static frontend, logged in, displayed runtime shape as `main | v0.0.1 | standard | static-dir-override | production | repos:healthy (0/1)`, and reached `就绪`.
- Chrome MCP console had no `error` or `warn` messages after readiness.
- Runtime happy-path smoke passed.
- Runtime recovery smoke passed.
- Docker release smoke built the local image, started the container with production auth material, verified `/api/node/role`, and verified production login.

## Fixes

- `scripts/smoke-docker-release.sh` now verifies production login after `/api/node/role` is healthy.
- `scripts/smoke-docker-release.sh` now supports `DEVE_DOCKER_SMOKE_AUTH_PASSWORD`; the default smoke password matches the default smoke `AUTH_PASS` hash.
- `scripts/check-release-baseline.sh` now guards the Docker smoke login check.
- `docs/dev-runbook.md` now states that Docker release smoke verifies production login, not only node-role health.

## Verification

Ran:

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/smoke-web-release-build.sh`
- production auth fail-closed startup smoke
- production configured startup + `/api/node/role` curl smoke
- production login success/failure curl smoke
- `DEVE_RUNTIME_BASE_URL=http://127.0.0.1:32132 DEVE_RUNTIME_SMOKE_REQUIRED=1 bash scripts/smoke-runtime-release-info.sh`
- Chrome MCP production frontend smoke
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `DEVE_DOCKER_SMOKE_REQUIRED=1 DEVE_DOCKER_SMOKE_PORT=32133 bash scripts/smoke-docker-release.sh`
- `npm audit --audit-level=high`
- `bash -n scripts/smoke-docker-release.sh scripts/check-release-baseline.sh`

Results:

- Release baseline: pass.
- Auth baseline: pass.
- Dev runbook baseline: pass.
- Web release build: pass.
- Production auth fail-closed: pass.
- Runtime release info: pass.
- Browser production frontend: pass.
- Runtime happy path: pass.
- Runtime recovery path: pass.
- Docker release smoke: pass.
- `npm audit --audit-level=high`: pass.

Residual note:

- `npm audit` reports one `moderate` Mermaid advisory. This is below the high/critical threshold checked in this batch and should be handled as dependency maintenance, not as a release smoke blocker.

## Decision

G1 is closed. Continue with Plugin Runtime Security Boundary Refresh.
