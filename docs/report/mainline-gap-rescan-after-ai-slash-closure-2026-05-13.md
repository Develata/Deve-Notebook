# Mainline Gap Rescan After AI Slash Closure - 2026-05-13

本报告记录当前 active queue 清空后的主线缺口复扫。`docs/plan/` 仍是唯一权威；本文件只记录执行队列判断与当前验证事实。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, `docs/acceptance-bindings.tsv`, current code, guard scripts, recent smoke reports.
- Non-goal: reopen `docs/plan/`,重新定义 native packaging gate, server-backed Settings API, Graph renderer, Calculation Runtime, plugin marketplace.

## Closed Since Previous Queue

The previous `mainline-gap-rescan-after-smoke-closure-2026-05-13.md` queue is closed:

- Auth security acceptance refresh: `auth-security-acceptance-refresh-2026-05-13.md`.
- Desktop Web shell browser smoke: `desktop-web-shell-browser-smoke-2026-05-13.md`.
- `.deveignore` watcher / scan user-facing smoke: `deveignore-watcher-scan-browser-smoke-2026-05-13.md`.
- Mobile residual interaction spot smoke: `mobile-residual-interaction-smoke-2026-05-13.md`.
- Source Control `CommitAndPush` browser smoke: `source-control-commit-and-push-browser-smoke-2026-05-13.md`.
- Plan-code mapping soft cleanup: `plan-code-mapping-soft-cleanup-2026-05-13.md`.
- Optional AI slash command smoke: `ai-slash-command-browser-smoke-2026-05-13.md`.

## Verification Snapshot

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`

Results:

- Acceptance binding: `93 automated / 62 feature / 29 manual / 0 unbound`.
- Architecture registry: `72 flows, 0 active drift`.
- Feature operation path check: pass.
- Plan coverage summary: `0` missing non-exempt `plan_ref`, `0` dangling `plan_ref`, `0` blocking violations.
- Release baseline: pass.
- Native track / native packaging gates: pass.
- Auth baseline and WS structured-error guard: pass.
- Storage/repo baseline: pass.

Fixed during scan:

- `docs/dev-runbook.md` missed `scripts/check-storage-repo-baseline.sh` in the current guard list.
- The missing entry made `scripts/check-dev-runbook-baseline.sh` fail.
- The runbook now lists the storage/repo baseline guard and the dev-runbook guard passes again.

## Current Boundary Check

- No blocking plan/code drift was found in this scan.
- Current practical delivery surfaces remain Web/server/Docker.
- Desktop/mobile native crates remain no-packaging skeletons; real Tauri/native packaging remains gate-closed.
- Settings server-backed API, Graph high-performance renderer, Calculation Runtime and plugin marketplace remain non-current work.
- Product MCP runtime remains retired; Chrome MCP remains only a browser verification tool.

## Next Gaps

### G1. Release / Production Runtime Verification Refresh

- Priority: P1.
- Plan basis: `15_release.md`, `09_auth.md`, `08_ui_design_01_web.md`.
- Acceptance basis: `REL-002`, `REL-006`, `REL-007`, `REL-008`, `AUTH-003`, `AUTH-012`.
- Current evidence: release baseline and runtime scripts exist; recent UI smoke used dev mode and isolated data roots, not a consolidated production/embedded/Docker pass after the queue closure.
- Gap: no current post-queue report proving embedded frontend, `/api/node/role`, production auth fail-closed / configured success, runtime happy/recovery scripts and Docker image smoke still align after the latest commits.
- Required output: one release/runtime report under `docs/report/`; targeted fixes only if smoke exposes stale embedded WASM, WS protocol mismatch, production auth drift, or Docker delivery drift.

### G2. Plugin Runtime Security Boundary Refresh

- Priority: P1.
- Plan basis: `17_plugins.md#plugin-runtime-boundary`, `10_ai_agent.md`.
- Acceptance basis: `PLUG-003`, `AI-005`, `AI-006`.
- Current evidence: Phase 1 fixed manifest entry path validation, nonce length validation, Rhai `eval` disablement, denied `env()` behavior, merge unwrap, and misleading Source Control fingerprint docs.
- Gap: no consolidated post-fix security boundary report for plugin host default-deny capability, path guard, loopback-only host boundary, structured plugin errors, trusted-cli default-off and sandbox tests.
- Required output: one plugin-security report under `docs/report/`; targeted fixes only if a boundary can be bypassed or a fail-closed path silently succeeds.

### G3. Protocol Error / Version Alignment Capture

- Priority: P2.
- Plan basis: `05_network.md`, `16_web_thin_client_ledger.md`, `11_i18n.md`.
- Acceptance basis: `NET-004`, `NET-012`, `NET-013`.
- Current evidence: reconnect and repo-scope browser recovery smoke is closed; `check-ws-structured-errors.sh` passes.
- Gap: no current focused report proving unsupported WS frame versions, malformed payloads and legacy frame rejection surface as structured protocol errors without locking the browser into a misleading disconnected state.
- Required output: protocol-error report with targeted server/web tests or Chrome MCP evidence; no broad network redesign.

### G4. Full Regression Gate After Targeted Fixes

- Priority: P2.
- Plan basis: engineering constitution, `15_release.md`.
- Acceptance basis: `REL-003`.
- Current evidence: targeted guard scripts are green.
- Gap: after G1-G3 targeted work, a final broader regression pass should be run once, not as the inner loop.
- Required output: record `cargo fmt --check`, targeted domain tests, and if resource budget allows `cargo test` or an explicit skipped rationale.

## Decision

Proceed with G1 first. The user-facing UI smoke queue is closed; the next highest-value risk is the production/embedded/Docker runtime boundary, especially because stale embedded frontend or protocol-version mismatch can make a locally passing dev UI fail in real deployment.
