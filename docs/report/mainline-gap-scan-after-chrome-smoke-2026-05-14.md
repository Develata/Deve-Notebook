# Mainline Gap Scan After Chrome Smoke - 2026-05-14

本报告记录 Chrome MCP isolated browser smoke 通过后的 fresh mainline implementation gap scan。`docs/plan/` 仍是唯一权威；本批次不修改 plan，不打开 native packaging gate。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/report/next-tasks.md`, `docs/features/`, `docs/acceptance-cases/`, current code, guard scripts, runtime smoke scripts.
- Non-goal: 修改 `docs/plan/`、引入 Tauri、打开 Desktop/Mobile packaging dependency gate、执行泛化重构。

## Baseline

Ran:

- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `bash scripts/check-i18n-formatting-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/check-release-audit-gate.sh`

Results:

- Plan coverage: `0` blocking violations, `17` existing soft size warnings.
- Architecture registry: `72` flows, `0` active drift.
- Acceptance bindings: `101` automated, `57` feature walkthrough, `29` manual, `0` unbound soft cases.
- Feature operation paths: pass.
- Network/Auth/Source Control/Rendering/Search: pass.
- Native track boundary: pass.
- Native packaging gate: pass; Desktop/Mobile packaging remains shell-only and feature-gated.
- Release/dev-runbook/storage/i18n/mobile/desktop/AI baselines: pass.
- Runtime happy/recovery smoke: pass.
- Release audit gate: `cargo-audit` unavailable in local diagnostic mode; npm audit found `0` vulnerabilities; script passed.

## Findings

### G1. No New Unblocked Current MUST Gap

- Priority: P0 verification.
- Plan basis: `docs/plan/` full current baseline.
- Finding: Current guard surface, runtime smoke, acceptance bindings, feature paths, architecture registry, release/native gates, and browser smoke are aligned. No new unblocked implementation gap was found.
- Decision: no immediate code change from this scan.

### G2. Plan Error-Code Catalog Patch Remains Permission-Gated

- Priority: blocked by process.
- Plan basis: `11_i18n.md`.
- Finding: `error-code-catalog-drift-review-2026-05-14.md` still identifies plan catalog entries that can be patched only when `docs/plan/` edits are explicitly authorized.
- Decision: keep `docs/plan/` unchanged.

### G3. Native Packaging Remains Explicitly Gated

- Priority: deferred.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Finding: Desktop/Mobile remain no-packaging skeletons and the packaging gate passes.
- Decision: do not start Tauri/Desktop/Mobile packaging without explicit gate-opening approval.

### G4. Full Regression Gate Is The Next Non-Implementation Step

- Priority: P1 verification.
- Plan basis: `15_release.md`, `docs/plan/AGENTS.md` validation discipline.
- Finding: The current narrow and medium guard set is healthy. Before starting a new implementation tranche without a concrete gap, the highest-signal next step is a full regression gate refresh.
- Decision: next unblocked validation should run full `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check`, unless the user explicitly opens the plan catalog patch, native packaging gate, or names a concrete Current MUST.

## Decision

No code change is required from this scan.

Next queue should not loop on fresh scans. If `docs/plan/` edits and native packaging remain closed, proceed to a full regression gate refresh or wait for a user-specified non-platform Current MUST.
