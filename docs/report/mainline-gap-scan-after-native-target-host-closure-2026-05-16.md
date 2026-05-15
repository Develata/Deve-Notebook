# Mainline Gap Scan After Native Target-host Closure - 2026-05-16

本报告记录 Desktop/Mobile target-host evidence 闭合后，对 core/web/server 主线的 `docs/plan` / features / acceptance / code 四向扫描。`docs/plan/` 仍是唯一真源；本批次不修改 plan，不打开 process runtime，不做 native shell 新能力。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, `docs/report/next-tasks.md`, current code, guard scripts, and runtime smoke scripts.
- Non-goal: 修改 `docs/plan/`、重跑 target-host workflows、声明 physical device readiness、引入 child-process runtime、执行泛化重构。

## Baseline

Ran:

- `scripts/check-architecture-registry.sh`
- `scripts/plan-coverage.sh`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`

Results:

- Architecture registry: `72` flows, `0` active drift.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Acceptance bindings: `101` automated, `57` feature walkthrough, `29` manual, `0` unbound soft cases.
- Feature operation paths: pass.
- Runtime happy-path smoke: pass.
- Runtime recovery smoke: pass.

## Findings

### G1. Formal Guards Are Green, But Some Current MUST Coverage Is Too Weak

- Priority: P0 coverage.
- Finding: `plan-coverage` and acceptance binding counts are green, but several Current MUST contracts are only weakly bound by generic help checks, broad baseline scripts, or stale manual cases.
- Decision: next batch should add precise coverage before adding unrelated new features.

### G2. Rendering Large-doc Acceptance Overstates Current Capability

- Priority: P1 docs/acceptance drift.
- Finding: `RENDER-LARGE-001` asserts `virtual_render_enabled true`, while `03_rendering.md` keeps complete virtual rendering outside current complete capability. Current code has large-doc search/prewarm/progressive infrastructure, not a complete virtual rendering engine.
- Decision: update acceptance wording and assertions to first-paint/progressive readiness only; do not claim virtual rendering until the plan gate is opened.

### G3. WebWrite Acceptance Cases Are Bound To The Wrong Behavior

- Priority: P1 acceptance drift.
- Finding: `WEBWRITE-FEAT-01/02/03` in features describe Pending -> Ack, Reject cleanup, and repo-scoped write readiness. The acceptance aliases currently point to pending navigation modal behavior.
- Decision: bind WebWrite acceptance to existing write/ack/reject/scope tests or add missing focused tests; keep pending navigation as its own case.

### G4. Modal Focus Contract Needs Code Coverage

- Priority: P1 user-facing UI contract.
- Finding: `08_ui_design.md#layout-navigation-and-focus` requires modal focus trap and focus restore. Shared `focus_scope` exists, but Settings, Pending Navigation, and Merge modals are not visibly wired to `role="dialog"`, `aria-modal`, trap, and restore behavior.
- Decision: implement a small shared modal focus wrapper or apply the existing focus helper consistently, then add focused tests/smoke checks.

### G5. Storage And Server Contracts Need Narrow Acceptance Bindings

- Priority: P1/P2 coverage.
- Finding: `.notegit/.git` segment ignore semantics, `.notegit-backup` negative case, ledger JSON Lines export, writeback-failure Ack, watcher overflow/reconcile/debounce, owner-only auth secret material, and repo catalog quarantine are not all explicitly represented in acceptance or baseline scripts.
- Decision: add targeted acceptance/baseline checks in small batches. Do not mix these with native target-host reruns.

### G6. Commands/Shortcuts Need Either Implementation Or Explicit Acceptance Narrowing

- Priority: P2 product surface.
- Finding: `12_commands.md` lists Source Control, Git bridge, P2P, and AI command palette surfaces. Current command registry covers core navigation, some P2P/Git read/notice commands, and AI chat toggle, but not every listed command surface as executable Web command.
- Decision: split implemented commands from reserved/unavailable commands in acceptance, then implement only the Current MUST surfaces that have backend authority and UI safety gates.

## Decision

The generic mainline scan is complete. The next executable work should be a small coverage/code alignment batch:

1. Correct large-doc and WebWrite acceptance drift.
2. Add or bind precise storage/server Current MUST coverage.
3. Fix modal focus trap wiring where code is missing.

Do not run another broad scan before these concrete gaps are closed.
