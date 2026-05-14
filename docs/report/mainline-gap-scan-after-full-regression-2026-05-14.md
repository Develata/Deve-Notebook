# Mainline Gap Scan After Full Regression - 2026-05-14

本报告记录 `full-regression-gate-refresh-2026-05-14.md` 通过后的主线缺口扫描。`docs/plan/` 仍是唯一真源；本批次不修改 plan，不打开 native packaging gate。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/report/next-tasks.md`, `docs/features/`, `docs/acceptance-cases/`, current code, guard scripts, and the latest full regression report.
- Non-goal: 修改 `docs/plan/`、引入 Tauri、打开 Desktop/Mobile packaging dependency gate、执行泛化重构。

## Baseline

Ran:

- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `scripts/check-architecture-registry.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-auth-unauthorized-state.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-source-control-smoke-hygiene.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/check-large-doc-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-release-audit-gate.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-dev-data-health-baseline.sh`
- `scripts/check-storage-repo-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-browser-prefs-boundary.sh`
- `scripts/check-i18n-hardcoded-baseline.sh`
- `scripts/check-i18n-formatting-baseline.sh`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-ui-desktop-baseline.sh`
- `scripts/check-ui-dashboard-refresh-baseline.sh`
- `scripts/check-ui-disconnect-baseline.sh`
- `scripts/check-ui-focus-baseline.sh`
- `scripts/check-ui-spa-routing-baseline.sh`
- `scripts/check-ui-token-baseline.sh`
- `scripts/check-ui-z-index-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-ws-structured-errors.sh`

Inherited from `full-regression-gate-refresh-2026-05-14.md`:

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `git diff --check`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`

Results:

- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Architecture registry: `72` flows, `0` active drift.
- Acceptance bindings: `101` automated, `57` feature walkthrough, `29` manual, `0` unbound soft cases.
- Feature operation paths: pass.
- Network/Auth/Source Control/Rendering/Search/Graph/Large-doc baselines: pass.
- Native track boundary and native packaging gate: pass.
- Release/dev-runbook/dev-data/storage/settings/browser-prefs baselines: pass.
- I18N/Mobile/Desktop/UI/AI/WS structured-error baselines: pass.
- Release audit gate: local diagnostic pass; `cargo-audit` unavailable locally, `npm audit` reports `0` vulnerabilities.
- Full regression and runtime smoke from the immediately preceding gate: pass.

## Findings

### G1. No New Unblocked Current MUST Gap

- Priority: P0 verification.
- Plan basis: full `docs/plan/` current baseline.
- Finding: Guard scripts, acceptance bindings, feature paths, architecture registry, latest full regression gate, and runtime smoke are aligned.
- Decision: no immediate code change is required from this scan.

### G2. Plan Error-Code Catalog Patch Remains Permission-Gated

- Priority: process-gated.
- Plan basis: `11_i18n.md`.
- Finding: `error-code-catalog-drift-review-2026-05-14.md` still identifies plan catalog entries that can be patched only when `docs/plan/` edits are explicitly authorized.
- Decision: keep `docs/plan/` unchanged.

### G3. Native Packaging Remains Explicitly Gated

- Priority: deferred.
- Plan basis: `08_ui_design_02_desktop.md`, `08_ui_design_03_mobile.md`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Finding: Desktop/Mobile remain no-packaging skeletons. The native packaging gate passes and should not be opened implicitly.
- Decision: do not start Tauri/Desktop/Mobile packaging in this batch.

### G4. Avoid Repeating Empty Scan Loops

- Priority: execution discipline.
- Plan basis: `00_engineering_constitution.md`, `docs/plan/AGENTS.md`.
- Finding: The current queue has no remaining unblocked implementation item after full regression and this scan.
- Decision: the next real implementation step requires one of three explicit inputs: authorize the plan error-code patch, open the native packaging gate, or select a concrete non-platform Current MUST.

## Decision

Current mainline is clean against the available guard surface. Do not run another generic gap scan unless code or plan changes. The next batch should be a concrete authorized item, not further validation churn.
