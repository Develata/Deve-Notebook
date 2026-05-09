<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# merge

## Purpose

Merge conflict resolution handlers. Supports both manual merge (user-driven conflict resolution) and peer merge (automatic reconciliation during sync).

## Key Files

| File | Description |
|------|-------------|
| `scope/mod.rs` | Merge scope bootstrap for single-repo sessions |
| `scope/test_support.rs` | Merge scope test fixtures |
| `scope/tests.rs` | Merge scope bootstrap tests |
| `scope/stale_tests.rs` | Merge stale scope binding tests |
| `manual.rs` | Manual merge handler — user resolves conflicts interactively |
| `manual_pending.rs` | Manual merge pending-operation handlers |
| `manual_support.rs` | Manual merge helper utilities |
| `peer.rs` | Peer merge handler — automatic during sync |
| `peer_apply/mod.rs` | Peer merge apply and conflict emission helpers |
| `peer_apply/tests.rs` | Peer merge apply tests |
| `peer_support/mod.rs` | Peer merge helper utilities |
| `peer_support/tests.rs` | Peer merge helper tests |
| `errors/mod.rs` | Merge-specific error helpers |
| `errors/classify_tests.rs` | Merge error classification tests |

## For AI Agents

### Working In This Directory

- `scope.rs` bootstraps local merge scope — shared bootstrap pattern with other handlers.
- Manual merge presents both sides to the user; peer merge auto-resolves using vector clocks.

<!-- MANUAL: -->
