# Mainline Gap Rescan After Native Port Validation - 2026-05-14

本报告记录 Native endpoint port validation 后的主线缺口复扫。`docs/plan/` 仍是唯一权威；本批次不修改 plan，不打开 native packaging gate。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/report/next-tasks.md`, current code, runtime smoke scripts, network/native/release guard scripts.
- Non-goal: 修改 `docs/plan/`、引入 Tauri、打开 mobile/desktop packaging dependency gate、执行泛化重构。

## Baseline

Ran:

- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`

Results:

- Network baseline: pass.
- Native track boundary: pass.
- Release baseline: pass.
- Dev runbook baseline: pass.
- Runtime happy path: pass.
- Runtime recovery path: pass.
- Plan coverage: `0` blocking violations, `17` existing soft size warnings.

## Findings

### G1. Native/Network Boundary Changes Did Not Regress Runtime Flow

- Priority: P0 verification.
- Plan basis: `05_network.md`, `08_ui_design_03_mobile.md`, `12_commands.md`.
- Finding: recent WS URL, query encoding, WS port override, and native endpoint port validation changes did not regress SyncHello, writer registration, edit ack, open/history readback, reconnect bootstrap, degraded-local write gate, stale-scope cleanup, read-only gate, status summary, or auth probe classification.
- Decision: no follow-up code change required from CLI/unit smoke.

### G2. Plan Error-Code Catalog Patch Remains Permission-Gated

- Priority: blocked by process.
- Plan basis: `11_i18n.md`.
- Finding: `error-code-catalog-drift-review-2026-05-14.md` still identifies plan catalog entries that can be patched only when `docs/plan/` edits are explicitly authorized.
- Decision: keep `docs/plan/` unchanged.

### G3. Native Packaging Remains Explicitly Gated

- Priority: deferred.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Finding: Desktop/Mobile remain no-packaging skeletons. This is still the correct state unless the native packaging gate is explicitly opened.
- Decision: do not start Tauri/Desktop/Mobile packaging in this batch.

## Decision

No unblocked mainline code gap was found by this pass.

The next unblocked validation step is a Chrome MCP isolated browser smoke refresh for the current Web runtime after the URL/endpoint boundary hardening batches. Implementation work should resume only if that smoke exposes a concrete bug, or after an explicit decision opens either the plan catalog patch or native packaging gate.
