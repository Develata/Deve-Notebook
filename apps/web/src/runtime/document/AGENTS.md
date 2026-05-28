<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-05-28 -->

# document

## Purpose

Document runtime band — pending overlay, ack/reject, and write-confirmation
for the thin-client write path. Phase B convergence target per
`docs/tasks/19_repo_refactor_blueprint.md` §3.3 and
`docs/report/runtime-convergence-audit-2026-05-28.md`. The pending overlay
(migrated here from `hooks/use_core/pending`), history resend, and
write-confirmation logic converge onto the typed contracts below.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Document band module entry |
| `pending.rs` | Pending local-edit overlay types and re-exports |
| `write_state.rs` | Formal `WriteConfirmation` state machine (Waiting/Committed/Rejected/WritebackFailed) |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `pending/` | Pending edit query/mutation helpers and server-history reconciliation |

<!-- MANUAL: -->
