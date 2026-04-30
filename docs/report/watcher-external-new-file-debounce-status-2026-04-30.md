# Watcher External New-File Debounce Status - 2026-04-30

## Scope

Closed the watcher hardening batch for external markdown files created directly under an active vault.

The observed failure mode was not a missing pending-side-table idempotency guard. `pending_fs::upsert_many` already preserves byte-stable rows for repeated semantic-equal signals. The missing boundary was higher up: `sync::handler` always emitted `FsChangeDetected` for an external added file even when the pending row was an idempotent skip.

## Changes

- `sync::pending::upsert` now returns whether the pending row was semantically changed.
- `sync::handler` emits add/delete refresh messages only when the pending side table actually changed.
- Duplicate added-file events now stay silent after the first stable pending row is created.
- Duplicate deleted-file events for the same tracked document now also avoid repeat refresh messages after the pending delete is stable.
- `sync::watcher::dispatch_test::dispatch_batch_suppresses_duplicate_external_added_message` covers the regression at the watcher dispatch callback boundary.

## Verification

- `cargo fmt --check`
- `cargo test -p deve_core sync::watcher::dispatch_test -- --nocapture`
- `cargo test -p deve_core`

## Current Boundary

This is a core/server-message boundary fix. It prevents repeated callback/WS refresh messages for duplicate semantic-equal watcher signals. It does not change ledger authority, staging, commit semantics, or full-scan behavior.
