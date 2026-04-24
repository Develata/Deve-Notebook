<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# merge

## Purpose

Merge conflict resolution handlers. Supports both manual merge (user-driven conflict resolution) and peer merge (automatic reconciliation during sync).

## Key Files

| File | Description |
|------|-------------|
| `scope.rs` | Merge scope bootstrap for single-repo sessions |
| `manual.rs` | Manual merge handler — user resolves conflicts interactively |
| `manual_pending.rs` | Manual merge pending-operation handlers |
| `manual_support.rs` | Manual merge helper utilities |
| `peer.rs` | Peer merge handler — automatic during sync |
| `peer_apply.rs` | Peer merge apply and conflict emission helpers |
| `peer_support.rs` | Peer merge helper utilities |
| `peer_support_test.rs` | Peer merge helper tests |
| `scope_stale_test.rs` | Merge scope stale binding tests |
| `errors.rs` | Merge-specific error types |
| `errors_classify_test.rs` | Merge error classification tests |
| `scope_test_support.rs` | Merge scope test fixtures |

## For AI Agents

### Working In This Directory

- `scope.rs` bootstraps local merge scope — shared bootstrap pattern with other handlers.
- Manual merge presents both sides to the user; peer merge auto-resolves using vector clocks.

<!-- MANUAL: -->
