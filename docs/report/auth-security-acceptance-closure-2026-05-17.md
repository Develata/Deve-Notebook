# Auth Security Acceptance Closure - 2026-05-17

本报告记录 `AUTH-005`、`AUTH-008`、`AUTH-009`、`AUTH-010` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `AUTH-005`: exact auth cookie name matching.
- `AUTH-008`: login rate limiting and brute-force fail-closed behavior.
- `AUTH-009`: minimized JWT payload.
- `AUTH-010`: structured WebSocket unauthorized response.

## Changes

- `scripts/check-auth-baseline.sh` now binds each case id to existing implementation checks and focused tests.
- No auth runtime behavior was changed.
- No new auth product scope was added.

## Verification

Ran:

- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- Auth baseline: passed.
- Acceptance bindings after this batch: automated `129`, feature walkthrough `54`, manual `17`, unbound `0`.

## Decision

Auth security acceptance residue is closed for the selected current cases.

Next batch: **AI Chat UX Acceptance Closure**.

First targets:

1. Bind `AI-001` and `AI-004` to existing AI Chat focused tests and browser-smoke evidence where possible.
2. Keep trusted-cli default-off and Native AI fallback boundaries unchanged.
3. Do not open external agent runtime beyond the existing explicit trusted-cli gate.
