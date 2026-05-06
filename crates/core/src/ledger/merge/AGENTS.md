<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# merge

## Purpose

Merge engine: computes diffs between branches, resolves conflicts using CRDT semantics, and applies merge results to the ledger.

## Key Files

| File | Description |
|------|-------------|
| `engine.rs` | Merge engine — orchestrates diff, conflict detection, resolution |
| `diff.rs` | Branch diff computation |
| `region.rs` | Merge region coalescing and conflict hunk construction |
| `types.rs` | Merge types (MergeResult, ConflictInfo) |
| `tests.rs` | Merge engine tests |

<!-- MANUAL: -->
