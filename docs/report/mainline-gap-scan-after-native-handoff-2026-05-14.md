# Mainline Gap Scan After Native Handoff - 2026-05-14

本报告记录 `native-target-host-handoff-2026-05-14.md` 落地后的主线缺口扫描。`docs/plan/` 仍是唯一真源；本批次不修改 plan，不打开 process runtime，不执行目标机打包。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/report/next-tasks.md`, `docs/features/`, `docs/acceptance-cases/`, current code, guard scripts, and runtime smoke scripts.
- Non-goal: 修改 `docs/plan/`、声明 macOS/Windows/iOS ready、打开 `Command::new`/spawn runtime、执行 iOS project generation。

## Baseline

Ran:

- `scripts/check-architecture-registry.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

Results:

- Architecture registry: `72` flows, `0` active drift.
- Acceptance bindings: `101` automated, `57` feature walkthrough, `29` manual, `0` unbound soft cases.
- Release/dev-runbook baselines: pass.
- Runtime happy-path smoke: pass.
- Runtime recovery smoke: pass.
- Native process adapter gate: pass; child-process runtime remains closed.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.

## Findings

### G1. No New Unblocked Current MUST Gap

- Priority: P0 verification.
- Finding: The local guard surface remains green after native target-host handoff.
- Decision: no immediate code implementation is required from this scan.

### G2. Target-host Work Remains the Only Native Packaging Blocker

- Priority: target-host blocked.
- Finding: Desktop macOS/Windows and Mobile iOS cannot be truthfully closed on the current Linux/WSL host.
- Decision: keep target-host package execution in `docs/report/next-tasks.md`; do not claim platform readiness locally.

### G3. Process Runtime Gate Must Stay Closed

- Priority: gate discipline.
- Finding: Android APK is verified, but Desktop macOS/Windows and Mobile iOS target-host evidence is still missing.
- Decision: `Command::new`, `tokio::process`, direct spawn, service ownership, and native authority writes remain out of scope.

## Decision

Current local mainline is clean against the available plan/code/acceptance gates. The next executable implementation step requires either:

- macOS/Windows/macOS-for-iOS target-host access, or
- an explicit non-platform Current MUST selected from `docs/plan/`.

Do not repeat generic gap scans unless code, plan, or target-host evidence changes.
