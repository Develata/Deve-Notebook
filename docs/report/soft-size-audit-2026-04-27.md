# Soft Size Audit 2026-04-27

This report records the current `scripts/plan-coverage.sh` single-file size warnings.
It is non-authoritative and does not override `AGENTS.md`; it only explains why no
mechanical split was applied in this acceptance-alignment batch.

## Result

- Hard fuse violations: 0
- Soft warnings reviewed: 7
- Decision: do not split files solely to satisfy the soft threshold.

`AGENTS.md` defines files over roughly 250 lines as architecture review warnings and
hand-written source over roughly 500 lines as hard fuse violations. The current
warnings are below the hard fuse and should be handled by cohesion, ownership, and
next-touch refactoring rather than arbitrary line-count slicing.

## Reviewed Warnings

| File | Lines | Assessment | Next action |
|---|---:|---|---|
| `crates/core/src/config.rs` | 261 | Central config schema and parsing are cohesive; only slightly above the threshold. | Keep; split only if settings domains diverge. |
| `crates/core/src/ledger/append_validate.rs` | 345 | Ledger append validation is a single invariant boundary. | Keep; extract only repeated validators. |
| `crates/core/src/ledger/merge/engine.rs` | 286 | Merge engine state machine is cohesive. | Keep unless new phases add distinct ownership. |
| `crates/core/src/ledger/metadata.rs` | 254 | Slightly above threshold; metadata surface is still compact. | Keep. |
| `crates/core/src/sync/mod.rs` | 328 | Public sync facade plus module wiring; current size is acceptable. | Consider moving facade helpers if new APIs are added. |
| `crates/core/src/sync/repo_scoped.rs` | 262 | Repo-scoped sync logic is cohesive and barely above threshold. | Keep. |
| `crates/core/tests/shadow_atomic_apply_test.rs` | 261 | End-to-end shadow atomicity scenario; contiguous context is useful. | Keep. |

## Completed Cleanup

`apps/cli/src/server/handlers/search.rs` was removed from the soft-warning list by
moving its feature-gated tests into `apps/cli/src/server/handlers/search_test.rs`.
The production handler now stays focused on request handling, scope resolution,
result shaping, and error classification.

`apps/cli/src/server/handlers/merge/peer_apply.rs` was removed from the
soft-warning list by moving its merge-conflict emission regression test into
`apps/cli/src/server/handlers/merge/peer_apply_test.rs`. The production file now
stays focused on peer merge apply, conflict emission, and completion broadcast
helpers.

`apps/cli/src/server/ws/receive_test.rs` was removed from the soft-warning list by
moving WS frame and legacy text protocol cases into
`apps/cli/src/server/ws/receive_frame_test.rs`. The original file now keeps the
shared receive fixture plus control-scope and rate-limit coverage.

`apps/cli/src/server/sync_transfer_scope_test.rs` was removed from the
soft-warning list by splitting sync transfer coverage into request/nonce,
push-source, and snapshot-source modules:
`sync_transfer_scope_test.rs`, `sync_transfer_push_test.rs`, and
`sync_transfer_snapshot_test.rs`.

`crates/core/src/sync/engine/manual_test.rs` was removed from the soft-warning list
by moving manual snapshot merge coverage into
`crates/core/src/sync/engine/manual_snapshot_test.rs`. The parent test module
keeps shared crypto/engine fixtures and non-snapshot manual merge coverage.
