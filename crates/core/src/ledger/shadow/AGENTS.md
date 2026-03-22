<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# shadow

## Purpose

Shadow branch access and management. Shadow branches hold remote peer state in separate Redb databases, enabling offline-first P2P sync.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry |
| `access.rs` | Shadow database read access |
| `management.rs` | Shadow lifecycle (create, delete, quarantine) |

## For AI Agents

### Working In This Directory

- Shadow repos are in `ledger/remotes/{peer_name}/`.
- Invalid shadows should be quarantined, not deleted.

<!-- MANUAL: -->
