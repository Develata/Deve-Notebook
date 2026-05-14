# Acceptance Stale Command Cleanup 2026-05-14

## Scope

- Cleaned stale acceptance commands after the current CLI/API surface changed.
- Kept `docs/plan/` unchanged.
- Added baseline guards so pseudo or removed commands cannot silently return.

## Changes

- `POS-004` now verifies rename identity through current `deve dump --path` output plus `watcher_pairs_rename_and_preserves_doc_identity`.
- `DIFF-001..003` now use current UTF-16 diff and merge-scope tests instead of removed dump/merge pseudo commands.
- `AUTH-009` now binds JWT claim minimization to current auth baseline and `issue_token_preserves_subject`.
- `REPO-FEAT-01` now uses the current `open_doc_scope` acceptance test.
- `STORE-010` now invokes the real integration test target `path_normalize_structure_test`.
- `scripts/smoke-web-release-build.sh` is marked as `CMD-007A`, not `REL-004`.

## Guards

- `scripts/check-acceptance-bindings.sh` rejects stale command strings across acceptance cases.
- `scripts/check-source-control-baseline.sh` rejects stale diff/merge pseudo commands.
- `scripts/check-auth-baseline.sh` rejects the removed `deve auth decode-jwt` surface.
- `scripts/check-storage-repo-baseline.sh` rejects the stale path normalization test filter.
- `scripts/check-cli-settings-baseline.sh` checks the web release smoke ownership label.

## Verification

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `cargo test -p deve_core --test path_normalize_structure_test -- --nocapture`
- `cargo test -p deve_core watcher_pairs_rename_and_preserves_doc_identity -- --nocapture`
- `cargo test -p deve_core compute_diff_uses_utf16_positions -- --nocapture`
- `cargo test -p deve_core issue_token_preserves_subject -- --nocapture`
