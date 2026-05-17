# Mainline Feature Selection After Full Regression - 2026-05-17

本报告记录 full regression gate 通过后的主线功能选择与第一批闭合结果。`docs/plan/` 未修改。

## Baseline

- Full regression gate: green.
- Acceptance bindings before this batch: automated `117`, feature walkthrough `54`, manual `29`, unbound `0`.
- Current blockers: none from plan coverage, architecture registry, feature operation paths, or runtime smoke.

## Selection

Selected first batch: **Network Acceptance Automation Closure**.

Rationale:

- Network cases cover WS handshake, repo scope, snapshot-first ordering, missing-fact transfer, snapshot fallback, indirect sync trust, structured protocol errors, and Unauthorized vs Disconnected.
- These paths are core infra for Web/server correctness and directly affect actual runtime testing.
- The batch does not open platform signing, physical-device release, native process runtime, Web Git writer, server-backed Settings API, or native authority writes.

Deferred candidates:

- Auth security acceptance closure: next highest priority, because it shares the session/WS boundary.
- AI Chat UX closure: user-facing, but lower risk than auth/session correctness.
- Rendering manual closure: localized UI verification, no current blocking drift.
- Plugin UI/security closure: important, but recent plugin hardening already has guard coverage.
- Positioning/terminology doc review: should stay manual-doc unless a concrete drift appears.

## Changes

- `scripts/check-network-baseline.sh` now binds `NET-005..013` to existing runtime smoke scripts, targeted tests, and structured-error/auth guard scripts.
- No product protocol, server, Web UI, or storage behavior was changed.

## Verification

Ran:

- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- Network baseline: passed.
- Acceptance bindings after this batch: automated `125`, feature walkthrough `54`, manual `21`, unbound `0`.

## Decision

Network manual acceptance residue is closed for current guard scope.

Next batch: **Auth Security Acceptance Closure**.

First targets:

1. Bind `AUTH-005`, `AUTH-008`, `AUTH-009`, `AUTH-010` to existing auth guard scripts and focused tests where possible.
2. Keep auth secret/dev fallback semantics fail-closed for production.
3. Do not add new auth product scope unless a concrete gap appears during the closure pass.
