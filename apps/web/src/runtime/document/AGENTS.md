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
| `confirm.rs` | Typed write-confirmation resolution — maps server `Ack`/`EditRejected`/echoed ops onto `WriteConfirmation` and clears the pending overlay (single path for the three former ack/reject/echo sites) |
| `pending.rs` | Pending local-edit overlay types and re-exports |
| `write_state.rs` | Formal `WriteConfirmation` state machine (Waiting/Committed/Rejected/WritebackFailed) |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `pending/` | Pending edit query/mutation helpers and server-history reconciliation |

## For AI Agents

### Consumer contract (typed boundary)

- This band is a **pure typed contract layer — it holds no Leptos signals.** `PendingLocalEdits` is plain data; signal ownership stays in `hooks/use_core` (`CoreSignals`).
- Consumers (`editor/sync`, `hooks/use_core/effects` dispatch) enter ONLY through the curated `pub` surface: `confirm::{commit_pending_edit, reject_pending_edit}`, `pending`'s re-exported functions/types, and `write_state::WriteConfirmation`. The inner `pending::ops` / `pending::history` modules are **private** — do not reach past the re-exports.
- Callers pass plain data in (read their signal via `get_untracked()`), so this band never depends back on `hooks/` (no `runtime -> hooks` inversion).
- The physical convergence of the upstream dispatch family (`browser_peer` / `browser_document`) is **deferred** per `docs/report/web-runtime-band-convergence-decision-2026-05-29.md` (§8 decision); `editor/sync` deliberately stays in `editor/` and consumes this contract.

<!-- MANUAL: -->
