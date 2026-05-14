# Scope Pref Serialization No-Panic

Date: 2026-05-14

## Scope

- `apps/web/src/hooks/use_core/scope_prefs.rs`
- `scripts/check-browser-prefs-boundary.sh`

## Contract

- `docs/plan/04_storage.md#browser-storage-layering`
- `docs/plan/06_repository.md#repo-scope-runtime`

## Change

- Replaced `expect("scope pref should serialize")` with an explicit `serialize_scope_pref` helper.
- Normal scope preference persistence still stores the same repo-name-only JSON payload.
- Unexpected serialization failure now skips the current preference write and logs a warning instead of panicking or clearing the previous preference.

## Verification

- `cargo test -p deve_web scope_pref -- --nocapture`
- `bash scripts/check-browser-prefs-boundary.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
