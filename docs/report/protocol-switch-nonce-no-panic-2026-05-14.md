# Protocol Switch Nonce No-Panic

Date: 2026-05-14

## Scope

- `apps/web/src/hooks/use_core/effects/message_protocol/control.rs`
- `scripts/check-auth-unauthorized-state.sh`

## Contract

- `docs/plan/09_auth.md#unauthorized-handling`
- `docs/plan/05_network.md#web-ws-runtime`

## Change

- Replaced the checked `switch_nonce.expect("checked above")` pattern with `let Some(switch_nonce) = switch_nonce else { return; }`.
- Kept the existing missing-nonce behavior unchanged: no pending scope switch is cleared without a matching switch nonce.
- Added an auth/protocol baseline guard to prevent reintroducing the panic-backed pattern.

## Verification

- `cargo test -p deve_web message_protocol -- --nocapture`
- `bash scripts/check-auth-unauthorized-state.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
