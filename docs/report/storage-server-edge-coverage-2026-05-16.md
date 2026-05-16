# Storage / Server Edge Coverage - 2026-05-16

本报告记录 Storage/Server Edge Coverage 批次。`docs/plan/` 未修改。

## Scope

- Watcher overflow / rescan normalization.
- Watcher debounce zero-window fail-closed boundary.
- Watcher modified-burst collapse.
- Remote repo catalog hard fail.
- Invalid shadow repo quarantine.
- Host identity key owner-only permission.

## Changes

- Added watcher backend tests for notify backend errors and explicit rescan flags.
- Added watcher lifecycle test proving zero debounce is rejected before watcher startup.
- Bound existing modified-burst, repo catalog fail-closed, shadow quarantine, and identity key permission tests to acceptance cases.
- Added `STORE-016`, `STORE-017`, and `AUTH-013` acceptance cases.
- Extended storage and auth baseline scripts to guard the new bindings.

## Verification

Ran:

- `cargo fmt`
- `cargo test -p deve_core notify_backend_error_requests_rescan -- --nocapture`
- `cargo test -p deve_core notify_rescan_flag_requests_rescan -- --nocapture`
- `cargo test -p deve_core watcher_rejects_zero_debounce_window -- --nocapture`
- `cargo test -p deve_core dispatch_batch_collapses_modified_burst_by_content_hash -- --nocapture`
- `cargo test -p deve_core remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing -- --nocapture`
- `cargo test -p deve_core remote_repo_listing_fails_closed_on_unexpected_non_redb_entry -- --nocapture`
- `cargo test -p deve_cli quarantines_nil_shadow_repo_into_invalid_peer_dir -- --nocapture`
- `cargo test -p deve_cli identity_key_permissions_are_corrected_to_owner_only -- --nocapture`
- `cargo test -p deve_cli identity_key_permissions_fail_closed_for_non_file -- --nocapture`
- `scripts/check-storage-repo-baseline.sh`
- `scripts/check-auth-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

Results:

- Storage/repo baseline: pass.
- Auth baseline: pass.
- Acceptance bindings: pass.
- Release baseline: pass.
- Plan coverage: pass with existing soft warnings only.
- Targeted core/CLI tests: pass.
- Format and diff hygiene: pass.

## Decision

Storage/Server Edge Coverage is closed. Next executable work is a mainline gap refresh after the edge-coverage bindings.
