# Mainline Refresh After Command Surface Closure - 2026-05-16

本报告记录 `Command Surface Reserved Boundary` 后的主线刷新。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, guard scripts, native gate scripts, runtime smoke scripts, and current code.
- Non-goal: 打开 Web Git writer、native process runtime、native authority writes、physical-device release readiness 或 target-host workflow rerun。

## Verification

Ran:

- `scripts/check-architecture-registry.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/plan-coverage.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-storage-repo-baseline.sh`
- `scripts/check-large-doc-baseline.sh`
- `scripts/check-i18n-hardcoded-baseline.sh`
- `scripts/check-i18n-formatting-baseline.sh`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-ui-desktop-baseline.sh`
- `scripts/check-ui-focus-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-release-audit-gate.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-desktop-package-preflight.sh`
- `scripts/check-mobile-platform-package-preflight.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-desktop-package-startup-smoke.sh`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`

Results:

- Architecture registry: `72` flows, `0` active drift.
- Acceptance bindings: `108` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Feature operation paths: pass.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Network/Auth/Source Control/Rendering/Search/Storage/Large-doc/I18N/Mobile/UI/AI/Graph baselines: pass.
- Release and dev-runbook baselines: pass.
- Release audit gate: pass; local `cargo-audit` unavailable in diagnostic mode, npm audit reported `0` vulnerabilities.
- Native track, Desktop preflight, Mobile preflight, native packaging, process adapter, and Desktop startup smoke gates: pass.
- Runtime happy-path smoke: pass.
- Runtime recovery smoke: pass.

## Findings

### G1. Command Surface Boundary Is Stable

- Priority: P0 verification.
- Finding: `CMD-004B` and `CMD-004C` are bound and covered; Git status/mirror/export remain CLI-only notice entries, and Source Control / AI unbound entries remain unavailable.
- Decision: no Web Git writer, background Git executor, AI backend switch shortcut, or Source Control commit shortcut was opened.

### G2. No New Unblocked Current MUST Gap

- Priority: P0 verification.
- Finding: The current guard surface is green after command-surface closure.
- Decision: do not start another broad cleanup batch without a specific user-facing target.

### G3. Platform Line Remains The Next Useful Frontier

- Priority: P1 planning.
- Finding: Docker release smoke, Desktop package evidence, Android emulator install/startup, and iOS simulator install/startup are already represented by scripts/reports. Remaining platform work is release-readiness triage, physical-device/signing/store decisions, and runbook/CI evidence quality.
- Decision: next batch should triage platform distribution readiness without opening native process runtime or native authority writes.

## Decision

Mainline Refresh After Command Surface Closure is closed.

Next executable work is Platform Distribution Readiness Triage: review Docker/Desktop/Android/iOS release evidence and select the smallest concrete platform batch that does not bypass shell-only/no-process/no-authority gates.
