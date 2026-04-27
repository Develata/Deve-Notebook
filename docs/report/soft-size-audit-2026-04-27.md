# Soft Size Audit 2026-04-27

This report records the current `scripts/plan-coverage.sh` single-file size warnings.
It is non-authoritative and does not override `AGENTS.md`; it only explains why no
mechanical split was applied in this acceptance-alignment batch.

## Result

- Hard fuse violations: 0
- Soft warnings reviewed: 11
- Decision: do not split files solely to satisfy the soft threshold.

`AGENTS.md` defines files over roughly 250 lines as architecture review warnings and
hand-written source over roughly 500 lines as hard fuse violations. The current
warnings are below the hard fuse and should be handled by cohesion, ownership, and
next-touch refactoring rather than arbitrary line-count slicing.

## Reviewed Warnings

| File | Lines | Assessment | Next action |
|---|---:|---|---|
| `apps/cli/src/server/handlers/merge/peer_apply.rs` | 257 | Cohesive peer-apply flow; just over the soft threshold. | Keep until next merge-handler edit. |
| `apps/cli/src/server/sync_transfer_scope_test.rs` | 334 | Scenario test where keeping setup and assertions together improves reviewability. | Keep unless duplicate setup grows. |
| `apps/cli/src/server/ws/receive_test.rs` | 299 | Protocol scenario test; splitting now would add indirection without reducing coupling. | Keep unless more WS receive cases are added. |
| `crates/core/src/config.rs` | 261 | Central config schema and parsing are cohesive; only slightly above the threshold. | Keep; split only if settings domains diverge. |
| `crates/core/src/ledger/append_validate.rs` | 345 | Ledger append validation is a single invariant boundary. | Keep; extract only repeated validators. |
| `crates/core/src/ledger/merge/engine.rs` | 286 | Merge engine state machine is cohesive. | Keep unless new phases add distinct ownership. |
| `crates/core/src/ledger/metadata.rs` | 254 | Slightly above threshold; metadata surface is still compact. | Keep. |
| `crates/core/src/sync/engine/manual_test.rs` | 290 | Manual sync scenario test with shared fixtures. | Keep unless fixture duplication emerges. |
| `crates/core/src/sync/mod.rs` | 328 | Public sync facade plus module wiring; current size is acceptable. | Consider moving facade helpers if new APIs are added. |
| `crates/core/src/sync/repo_scoped.rs` | 262 | Repo-scoped sync logic is cohesive and barely above threshold. | Keep. |
| `crates/core/tests/shadow_atomic_apply_test.rs` | 261 | End-to-end shadow atomicity scenario; contiguous context is useful. | Keep. |

## Completed Cleanup

`apps/cli/src/server/handlers/search.rs` was removed from the soft-warning list by
moving its feature-gated tests into `apps/cli/src/server/handlers/search_test.rs`.
The production handler now stays focused on request handling, scope resolution,
result shaping, and error classification.
