# Watcher Rename-Pair Debounce Status - 2026-04-30

## Scope

Closed the core watcher hardening batch for duplicate semantic-equal rename-pair events.

The previous external new-file debounce fix stopped repeated add/delete refreshes in normal `FsEventHandler` paths. Rename-pair dispatch still had a separate unconditional broadcast path: after `pending_rename::upsert_external_rename`, `dispatch_rename` always emitted deleted/added `FsChangeDetected` messages even when both pending side-table rows were idempotent skips.

## Changes

- `sync::pending_rename::upsert_external_rename` now returns whether either side of the rename pair changed.
- `sync::watcher::dispatch_rename` suppresses duplicate deleted/added callback messages when the pending rename pair is unchanged.
- `sync::handler::record_external_rename` applies the same guard for direct handler rename detection.
- `sync::watcher::dispatch_test::dispatch_batch_suppresses_duplicate_rename_pair_messages` covers the regression at callback boundary.

## Verification

- `cargo fmt --check`
- `cargo test -p deve_core sync::watcher::dispatch_test -- --nocapture`
- `cargo test -p deve_core`

## Current Boundary

This is a core/server-message boundary fix. It does not change authority facts, staging, commit coalescing, Source Control target resolution, or fallback behavior when rename pairing cannot identify a tracked document. If a rename cannot be safely paired, the existing fail-closed/degraded behavior remains unchanged.
