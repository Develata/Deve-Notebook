<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# document

## Purpose
Document content operations: opening documents (loading snapshot + delta ops), editing (applying OT Insert/Delete operations), retrieving edit history, and managing confirmed operation state. All operations validate repo scope before proceeding.

## Key Files
| File | Description |
|------|-------------|
| `open.rs` | OpenDoc handler: loads content snapshot via base + delta, sends `Snapshot` message |
| `edit.rs` | Edit handler: applies OT operations, rejects edits on remote branches, broadcasts `ConfirmedOp` |
| `snapshot.rs` | Snapshot building: retrieves base snapshot + pending delta ops |
| `history.rs` | Document edit history retrieval |
| `confirmed.rs` | Confirmed operation tracking and emission |
| `errors/mod.rs` | Document error classification (maps to `ServerErrorCode` using `error_classify` patterns) |
| `errors/tests.rs` | Document error classification tests |

## For AI Agents

### Working In This Directory
- `resolve_document_scope` bootstraps scope for unbound sessions before loading content.
- Edit operations on remote branches are rejected with `ScRemoteBranchReadonly`.
- Error classification maps storage corruption, missing projections, and locked DBs to specific error codes.
- Snapshot loading is performance-sensitive; uses base snapshot + delta ops to avoid replaying full history.

<!-- MANUAL: -->
