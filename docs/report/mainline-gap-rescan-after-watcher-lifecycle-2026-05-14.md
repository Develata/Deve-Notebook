# Mainline Gap Rescan After Watcher Lifecycle - 2026-05-14

本报告记录 watcher lifecycle stop/start race 修复后的主线基线复验。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/report/next-tasks.md`, current code, runtime smoke scripts, release/native guard scripts, Web package audit.
- Non-goal: 修改 `docs/plan/`、打开 native packaging gate、引入 Tauri 或移动端 packaging dependency。

## Baseline

Ran:

- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `npm audit --audit-level=moderate` in `apps/web`
- `npm view mermaid version` in `apps/web`

Results:

- Runtime happy path: pass.
- Runtime recovery path: pass.
- Plan coverage: `0` blocking violations, `17` existing soft size warnings.
- Release baseline: pass.
- Dev runbook baseline: pass.
- Native track boundary: pass.
- Native packaging gate: pass.
- Web dependency audit: `0` vulnerabilities at moderate threshold.
- Mermaid published version: `11.15.0`; current lockfile already uses `11.15.0`.

## Findings

### G1. Runtime Baseline Remains Healthy

- Priority: P0 verification.
- Plan basis: `05_network.md`, `07_diff_logic.md`, `12_commands.md`, `15_release.md`.
- Finding: watcher lifecycle registry changes did not regress WS handshake, writer registration, edit ack, open/history readback, reconnect bootstrap, degraded-local write gate, stale scope cleanup, read-only gate, status summary, or auth probe classification.
- Decision: no follow-up code change required.

### G2. Docker Release Smoke Already Closed

- Priority: Closed.
- Plan basis: `15_release.md`, `REL-002`, `REL-008`.
- Finding: `docker-release-smoke-freshness-2026-05-14.md` already reran current Dockerfile smoke after the compose split and fixed the WSL bind-mount storage issue by using a temporary Docker named volume.
- Decision: do not rerun Docker smoke in this batch unless Dockerfile, compose, auth, or release runtime changes again.

### G3. Dependency Maintenance Advisory Closed

- Priority: Closed.
- Plan basis: `15_release.md` release checklist.
- Finding: previous Mermaid moderate advisory is no longer present under the current lockfile. `npm audit --audit-level=moderate` returns `0` vulnerabilities, and `npm view mermaid version` reports `11.15.0`, matching current `apps/web/package-lock.json`.
- Decision: no dependency patch required.

### G4. Native Packaging Remains Explicitly Gated

- Priority: Deferred.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Finding: default Desktop/Mobile crates remain no-packaging skeletons and packaging dependency gates pass.
- Decision: do not start Tauri/Desktop/Mobile packaging without an explicit gate-opening decision.

### G5. Plan Error-Code Catalog Patch Remains Permission-Gated

- Priority: Blocked by process.
- Plan basis: `11_i18n.md`.
- Finding: `error-code-catalog-drift-review-2026-05-14.md` identified plan catalog entries that can be patched only when `docs/plan/` edits are explicitly authorized.
- Decision: keep `docs/plan/` unchanged in this batch.

## Decision

No unblocked mainline code gap was found in this pass. The next unblocked validation step is a Chrome MCP isolated browser smoke refresh for the current Web runtime. Implementation work should resume only after that smoke exposes a concrete bug, or after an explicit decision opens either the plan catalog patch or native packaging gate.
