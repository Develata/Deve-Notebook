# Remote Repo Selector Readable No-Panic

Date: 2026-05-14

## Scope

- `crates/core/src/ledger/manager/remote_repo_select.rs`
- `scripts/check-storage-repo-baseline.sh`

## Contract

- `docs/plan/06_repository.md#repo-selector-resolution-contract`

## Change

- Replaced `expect("validated readable")` in remote repo selector resolution with a local `let Some(info) = ... else` guard.
- Preserved existing fail-closed behavior: unreadable shadow metadata returns the same broken-shadow-repo error instead of panicking.
- Added a storage/repo baseline guard to prevent reintroducing the panic-backed assumption.

## Verification

- `cargo test -p deve_core resolve_remote_repo_entry -- --nocapture`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`
