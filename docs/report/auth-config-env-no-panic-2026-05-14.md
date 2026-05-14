# Auth Config Env No-Panic

Date: 2026-05-14

## Scope

- `crates/core/src/security/auth/config.rs`
- `scripts/check-auth-baseline.sh`

## Contract

- `docs/plan/09_auth.md#auth-config`

## Change

- Replaced `expect("checked above")` in `AuthConfig::from_env` with explicit pattern matching over `AUTH_SECRET` and `AUTH_PASS`.
- Production mode still fails closed when either required variable is missing.
- Development mode still falls back to explicit insecure dev defaults only when `DEVE_ENV=development`.
- Added a production missing-secret/password regression test and auth baseline guard.

## Verification

- `cargo test -p deve_core missing_secret_or_password -- --nocapture`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
