# Mainline Gap Rescan - 2026-05-13

## Scope

- Source of truth: `docs/plan/`.
- Inputs:
  - `docs/features/`
  - `docs/acceptance-cases/`
  - `docs/acceptance-bindings.tsv`
  - current guard scripts
  - recent `docs/report/*2026-05-12.md` and `docs/report/*2026-05-13.md`
- Excluded as closed by recent smoke:
  - AI BUILD Apply
  - Merge Conflict UI
  - Rendering interaction spot smoke
  - Settings / Extensions reserved UI

This report does not modify `docs/plan/`.

## Verification Snapshot

Ran:

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/plan-coverage.sh`
- `scripts/check-network-baseline.sh`
- `scripts/check-storage-repo-baseline.sh`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-ui-dashboard-refresh-baseline.sh`
- `scripts/check-graph-baseline.sh`

Results:

- Acceptance binding: `93 automated / 62 feature / 29 manual / 0 unbound`.
- Architecture registry: `72 flows, 0 active drift`.
- Plan coverage: `0` blocking violations.
- Candidate-domain baseline scripts pass after guard drift cleanup.

Guard cleanup:

- `check-network-baseline.sh` now matches lifecycle-aware `try_set` status writes.
- `check-ui-dashboard-refresh-baseline.sh` now matches lifecycle-aware `try_set` connection epoch writes.

## Closed Since Last Selection

- `AI-003` browser BUILD Apply path: closed by `ai-build-apply-browser-smoke-2026-05-12.md`.
- `DIFF-003/004` conflict action path: closed by `merge-conflict-ui-browser-smoke-2026-05-13.md`.
- Rendering interaction spot checks: closed by `rendering-interaction-spot-smoke-2026-05-13.md`.
- `SET-006/007` and `PLUG-002` reserved UI path: closed by `settings-extensions-reserved-ui-browser-smoke-2026-05-13.md`.

## Findings

### G1. Network / Repo Scope Browser Recovery Remains Highest Risk

Priority: P1.

Sources:

- `docs/plan/05_network.md`
- `docs/features/05_network.md`
- `docs/features/operations/net_sync_handshake.md`
- `docs/acceptance-cases/06_network.md`

Current evidence:

- Protocol, frame, writer gate, auth-state split and repo-scoped handshake have code tests and static guards.
- Runtime happy/recovery scripts cover targeted server-side and hook-level flows.

Gap:

- There is no recent Chrome smoke for backend unavailable -> reconnect -> recover in the live Web shell.
- There is no recent Chrome smoke for switching between two repos and confirming stale scope messages cannot drive the new scope.

Selection rationale:

- This area can cause user-visible lockout, stale writes, or cross-repo state pollution.
- It should precede broader UI polish smoke.

### G2. Repo / Remote Spectator Read-only UI Needs Browser Evidence

Priority: P1.

Sources:

- `docs/plan/06_repository.md`
- `docs/features/06_repository.md`
- `docs/acceptance-cases/14_operation_flow_refs.md`

Current evidence:

- Repo scope, remote listing, switcher and readonly mutation gates have CLI/server tests.
- Source Control smoke covered local pending/stage/commit, not remote/spectator UI state.

Gap:

- No recent browser evidence that remote/spectator scope is visibly read-only.
- No recent browser evidence that create/edit/stage/commit controls are blocked or disabled in that scope.

Selection rationale:

- This directly protects `.notegit` / ledger authority boundaries.
- It is adjacent to G1 and can reuse the same isolated multi-repo setup.

### G3. Browser Storage / Projection Degraded Write Gate Is Not Browser-Smoked

Priority: P1.

Sources:

- `docs/plan/04_storage.md`
- `docs/features/04_storage.md`
- `docs/acceptance-cases/07_storage_repo.md`

Current evidence:

- `STORE-011/013` have unit, CLI and static guard coverage.
- `storage-repo-acceptance-drift-2026-05-12.md` fixed CLI acceptance drift.

Gap:

- No recent Chrome smoke proving degraded browser storage or degraded projection is visible as read-only in UI.
- No recent Chrome smoke proving `RegisterWriter`, edit and Source Control mutations remain blocked from the user surface.

Selection rationale:

- This is a data-safety gate, not a future feature.
- It should be validated before adding platform or renderer work.

### G4. Mobile Web Shell Needs A Current Narrow-Viewport Smoke

Priority: P2.

Sources:

- `docs/plan/08_ui_design_03_mobile.md`
- `docs/features/08_ui_design_03_mobile.md`
- `docs/acceptance-cases/05_ui.md`
- `docs/acceptance-cases/13_ui_mobile_chat_regression.md`

Current evidence:

- Mobile baseline script and unit tests cover viewport mapping, drawers, search sheet, bottom bar, touch target and chat regressions.
- Recent reports mention mobile branch coverage, but not a full 375x812 Chrome smoke.

Gap:

- No current end-to-end narrow viewport browser report for drawer, More menu, search sheet, bottom bar and chat open/close.

Selection rationale:

- Mobile is increasingly important, but it should follow scope/write safety smoke.

### G5. Command Surfaces Need A Routing Smoke

Priority: P2.

Sources:

- `docs/plan/12_commands.md`
- `docs/features/12_commands.md`
- `docs/features/operations/command_surface_action_routing.md`
- `docs/acceptance-cases/11_commands_settings.md`

Current evidence:

- Static guard confirms `Ctrl+P`, `Ctrl+Shift+P`, `Ctrl+Shift+K`, Command Palette and Branch Switcher existence.
- Several command targets have unit tests.

Gap:

- No recent browser smoke for keyboard shortcut routing, command search, execution, cancellation and branch switcher visibility.

Selection rationale:

- Recent browser smoke found real command/action routing bugs, so this is a useful regression surface.

### G6. Git Mirror Web Boundary Needs Read-only Notice Smoke

Priority: P2.

Sources:

- `docs/plan/07_diff_logic.md`
- `docs/features/12_commands.md`
- `docs/acceptance-cases/04_diff.md`

Current evidence:

- CLI bridge and source-control baseline scripts enforce Web Git writer absence.
- Unit tests cover CLI-only notices and repair review policy.

Gap:

- No recent browser evidence for Git Import/Push/Repair Command Palette notices and read-only repair review states.

Selection rationale:

- The main risk is accidentally turning Web UI into a Git writer, which plan forbids.

### G7. Dashboard / Graph Are Good Later Smoke Targets

Priority: P2.

Sources:

- `docs/plan/14_tech_stack.md`
- `docs/plan/15_release.md`
- `docs/acceptance-cases/12_tech_release.md`

Current evidence:

- Dashboard and Graph baseline scripts pass.
- Graph current boundary is read-only projection, not high-performance renderer.

Gap:

- Dashboard lacks a recent browser wait-for-live-metrics report.
- Graph lacks a recent browser report for summary counts, empty/degraded states and renderer-gate copy.

Selection rationale:

- These are visible quality gates, but lower data-integrity risk than G1-G3.

## Next Execution Queue

1. Network / repo scope browser recovery smoke.
2. Repo / remote spectator read-only UI smoke.
3. Browser storage / projection degraded write-gate smoke.
4. Mobile Web shell narrow-viewport smoke.
5. Command Palette / Quick Open / Branch Switcher routing smoke.
6. Git mirror CLI-only notice / read-only repair review smoke.
7. Dashboard SystemMetrics browser smoke.
8. Graph read-only projection panel browser smoke.

First execution target: G1. It is the highest-risk mainline gap because it validates reconnect, scope isolation and write-gate behavior in the live Web shell.
