# Auth Hardening Batch - 2026-05-14

本报告记录 mainline gap rescan 后的第一批 auth hardening。`docs/plan/` 未修改。

## Scope

- Plan basis: `09_auth.md#auth-config`, `09_auth.md#auth-http-endpoints`, `09_auth.md#jwt-cookie-contract`, `09_auth.md#password-hashing`.
- Code scope: auth config loading, password PHC validation, JWT issuing, login/session tests.
- Non-goal: change cookie contract, change middleware semantics, add a new auth provider, or alter production/dev mode policy.

## Changes

- `jwt::issue_token` now accepts the authenticated subject and stores it in `Claims.sub`.
- Login now issues tokens with `config.username`, so non-default `AUTH_USER` is preserved through `/api/auth/me`.
- `AuthConfig::from_env` validates `AUTH_PASS` as an Argon2 PHC string before accepting production startup.
- Password verification reuses the same Argon2 PHC parser, avoiding a separate late-only invalid-hash path.
- `scripts/check-auth-baseline.sh` now guards the startup PHC validation string and subject-based JWT issuing path.

## Verification

Ran:

- `cargo fmt --check`
- `cargo test -p deve_core auth -- --nocapture`
- `cargo test -p deve_cli auth -- --nocapture`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo clippy -p deve_core -p deve_cli --all-targets -- -D warnings`

Results: pass.

## Residual Work

- WS text-frame debug gate remains next.
- Mobile PendingAck scope filtering remains pending.
- Source Control HTTP scope gate and watcher lifecycle remain larger runtime-design followups.
