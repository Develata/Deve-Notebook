# Mainline Gap Rescan After Smoke Queue Closure - 2026-05-13

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, current code, guard scripts, recent smoke reports.
- Non-goal: reopen `docs/plan/`, native packaging, Tauri dependency gate, Graph renderer, server-backed Settings API, Calculation Runtime, plugin marketplace.

## Closed From Previous Queue

The previous `mainline-gap-rescan-2026-05-13.md` queue is closed by recent reports:

- Network / repo scope browser recovery: `network-repo-scope-browser-recovery-smoke-2026-05-13.md`.
- Remote spectator read-only UI: `remote-spectator-readonly-ui-smoke-2026-05-13.md`.
- Browser storage / projection degraded write-gate: `browser-storage-projection-degraded-write-gate-smoke-2026-05-13.md`.
- Mobile narrow viewport shell: `mobile-web-shell-narrow-viewport-smoke-2026-05-13.md`.
- Command Surface routing: `command-surface-routing-smoke-2026-05-13.md`.
- Git mirror CLI notice / readonly repair review: `git-mirror-cli-notice-readonly-repair-smoke-2026-05-13.md`.
- Dashboard metrics and Graph readonly projection panel: `dashboard-system-metrics-browser-smoke-2026-05-13.md`, `graph-readonly-projection-panel-smoke-2026-05-13.md`.

## Current Boundary Check

- Code-side P0/P1 current-contract defect: none found in this rescan.
- Native packaging remains post-gate, not a current runtime gap.
- Desktop and Mobile crates are current no-packaging skeletons.
- Real Tauri dependencies and native child-process runtime remain blocked by gate scripts.
- Web/server/Docker remain the current practical delivery surfaces.

Verification:

- `bash scripts/check-native-track-boundary.sh`: passed.
- `bash scripts/check-mobile-baseline.sh`: passed.
- `bash scripts/check-auth-baseline.sh`: passed.
- `bash scripts/check-ui-desktop-baseline.sh`: passed.
- `cargo test -p deve_desktop -- --nocapture`: 21 passed.
- `cargo test -p deve_mobile -- --nocapture`: 19 passed.
- `cargo test -p deve_core native_adapter -- --nocapture`: 30 passed.

## Next Gaps

### G1. Auth Security Acceptance Refresh

- Priority: P1.
- Plan basis: `09_auth.md` is Current MUST.
- Acceptance basis: `AUTH-003` through `AUTH-010`, plus `AUTH-012`.
- Current state: no known auth defect in this rescan; `AUTH-011` session-expired vs disconnected browser smoke is closed, but broader cookie / CORS / CSRF / rate-limit / JWT / WS unauthorized / status endpoint behavior needs a fresh post-refactor acceptance pass.
- Required output: one auth security report under `docs/report/`, targeted fixes only if the refresh finds defects.

### G2. Desktop Web Shell Browser Smoke

- Priority: P2.
- Plan basis: Web wide viewport MUST follow Desktop shell contract.
- Acceptance basis: `UI-DESK-001`, `UI-DESK-002`, `UI-DESK-003`.
- Current state: script and unit-level guards exist; recent Chrome MCP queue has no dedicated Desktop wide-screen layout/resizer/search smoke.
- Required output: browser smoke for desktop diff scroll sync, sidebar/right-panel/outer-gutter persistence, and search mode prefix routing.

### G3. `.deveignore` User-Facing Watcher / Scan Smoke

- Priority: P2.
- Plan basis: watcher, directory rescan, and startup scan MUST consistently ignore `.deveignore` paths.
- Acceptance basis: storage watcher and Source Control pending visibility.
- Current state: automated storage drift checks exist; a browser/source-control user-facing smoke should confirm ignored markdown never appears as pending work.
- Required output: browser or CLI-plus-browser smoke report with temporary repo fixture.

### G4. Mobile Residual Interaction Spot Smoke

- Priority: P2.
- Plan basis: mobile Web shell remains Current UI Contract.
- Acceptance basis: uncovered or only guard-level cases such as `UI-MOB-004`, `UI-MOB-006`, `UI-MOB-013`, `UI-MOB-014`, `UI-MOB-017`, `UI-MOB-018`.
- Current state: broad narrow-viewport smoke closed the main shell path, drawers, search sheet, bottom bar, AI fullscreen, touch targets, and console/network health.
- Required output: focused mobile browser smoke for keyboard toolbar, search-result scroll isolation, AI readability/error recovery, and mobile diff visibility/close behavior.

### G5. Source Control `CommitAndPush` Browser Smoke

- Priority: P2.
- Plan basis: Source Control publish actions are current source-control surface, while Git mirror publish remains CLI-only.
- Acceptance basis: `DIFF-FEAT-02` / `flow.sc.commit-and-push`.
- Current state: static guard confirms `ClientMessage::CommitAndPush`, server route, handler, and `CommitAck` completion contract; previous Source Control smoke covered stage / unstage / commit / reload, not the `Commit & Push` button path.
- Required output: browser smoke proving `Commit & Push` sends the scoped message, returns the current `CommitAck` completion, and does not imply Web Git mirror push authority.

### G6. Plan-Code Mapping Soft Cleanup

- Priority: P2.
- Plan basis: plan-code bijection remains a current engineering constraint.
- Current state: `plan-coverage.sh` has zero blocking violations, zero dangling `plan_ref`, zero unbound acceptance cases, and zero hard fuse violations. It still reports 56 non-exempt source modules without `plan_ref` and 17 soft line warnings.
- Required output: small batches only; add accurate `plan_ref` annotations or justify cohesive soft-size files without mechanical splitting.

### G7. Optional AI Slash Command Smoke

- Priority: P3.
- Plan basis: AI is Optional Product Layer.
- Acceptance basis: `AI-002` `/plan` and `AI-004` `/agents`.
- Current state: Native AI positive path and `/build Apply` are closed; `/plan` and `/agents` remain lower-priority UX smoke candidates.
- Required output: optional smoke only after Current MUST and Current UI Contract gaps above are closed.

## Decision

Proceed with G1 first. It is the highest-priority remaining gap because it belongs to Current MUST, is security-sensitive, and can be validated without native packaging or broad UI redesign.
