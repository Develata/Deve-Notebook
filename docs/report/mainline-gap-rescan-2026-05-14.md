# Mainline Gap Rescan - 2026-05-14

本报告记录 `final-regression-gate-2026-05-13.md` 后的下一轮主线 gap scan。`docs/plan/` 仍是唯一权威；本文件只记录当前代码与报告事实。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/operations/`, `docs/acceptance-cases/`, `docs/report/next-tasks.md`, current code and guard scripts.
- Non-goal: reopen native packaging gate, introduce Tauri dependency, implement mobile store packaging, or change `docs/plan/`.

## Baseline

Ran:

- `cargo test -p deve_web disconnected_lockdown -- --nocapture`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-release-audit-gate.sh`

Results:

- Plan coverage: `0` blocking violations, `17` existing soft size warnings.
- Native track boundary: pass.
- Native packaging gate: pass.
- Desktop/Mobile no-packaging baseline: pass.
- Release baseline: pass.
- Release audit gate: local diagnostic mode pass; `cargo-audit` is unavailable locally, and `npm audit` reports one moderate Mermaid advisory below the current high/critical release threshold.

## Findings

### G1. Docker Compose Release Shape Drift

- Priority: P1.
- Plan basis: `15_release.md#run-with-docker-compose`.
- Finding: `docker-compose.yml` mixed release and local-build semantics. It used local `build`, `container_name: deve-note`, `restart: unless-stopped`, and a named data volume, while the plan's release compose uses the published GHCR image, `container_name: deve-server`, `restart: always`, and `./data:/data`.
- Decision: root `docker-compose.yml` is production release compose. Local Dockerfile build now belongs to `docker-compose.dev.yml`.

Fixes:

- Updated `docker-compose.yml` to use `ghcr.io/develata/deve-notebook:latest`, `container_name: deve-server`, `restart: always`, and `./data:/data`.
- Added `docker-compose.dev.yml` for local Dockerfile build with a named dev volume.
- Updated `scripts/check-release-baseline.sh` to guard the release/dev compose split.
- Updated `docs/dev-runbook.md` and `scripts/check-dev-runbook-baseline.sh` with the production/dev compose distinction.

Verification:

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `AUTH_SECRET=... AUTH_PASS=... docker compose -f docker-compose.yml config`
- `AUTH_SECRET=... AUTH_PASS=... docker compose -f docker-compose.dev.yml config`

Results: pass.

### G2. Docker Release Smoke Freshness

- Priority: P2.
- Plan basis: `15_release.md`, `REL-002`.
- Finding: Docker release smoke passed in the 2026-05-13 report, but this rescan did not rebuild the image after the compose split.
- Required output: run `DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh` in a dedicated batch and record whether the current Dockerfile still produces a login-capable production container.

### G3. Native Packaging Remains Post-Gate

- Priority: Deferred.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Finding: Desktop/Mobile are correctly implemented as no-packaging skeletons with native adapter contracts, not full Tauri apps.
- Decision: do not open Tauri/Desktop/Mobile packaging in this batch. The correct precondition is a separate gate design and review.

### G4. Dependency Maintenance Advisory

- Priority: P3.
- Plan basis: `15_release.md` release checklist.
- Finding: local `npm audit --audit-level=high` passes, but plain `npm audit` reports a moderate Mermaid advisory.
- Required output: handle as dependency maintenance, not a release blocker, unless the release threshold changes.

## Decision

G1 is the only implemented gap in this batch. Continue with G2 next: run Docker release smoke against the current Dockerfile and record a fresh report. Do not start Desktop/Mobile Tauri work until release/container shape is stable and a native packaging gate design is explicitly opened.
