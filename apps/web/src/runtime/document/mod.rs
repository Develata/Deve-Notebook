//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 10_rendering#document-authority-bridge
//!
//! Document runtime band — open / pending / ack-reject / navigation for the
//! thin-client write path.
//!
//! Phase B convergence target per `docs/tasks/19_repo_refactor_blueprint.md`
//! §3.3 and `docs/report/runtime-convergence-audit-2026-05-28.md`. The pending
//! overlay now lives here (`pending`, migrated out of `hooks/use_core/pending`
//! in Phase B step 2); history resend and the per-edit write-confirmation logic
//! still scattered across `hooks/use_core/callbacks_sync/write` and
//! `editor/sync` converge here onto the typed contracts below.
//!
//! # Consumer contract (typed boundary)
//!
//! This band is a **pure typed contract layer**: it holds no Leptos signals.
//! `PendingLocalEdits` is plain data (`HashMap`); signal ownership stays in
//! `hooks/use_core` (`CoreSignals`). Consumers — `editor/sync` and the
//! `hooks/use_core/effects` dispatch handlers — enter ONLY through the curated
//! `pub` surface of the three submodules below:
//!
//! - `confirm` — `commit_pending_edit` / `reject_pending_edit` (the single
//!   typed entry that applies a server confirmation to the overlay).
//! - `pending` — the re-exported query/mutation/reconcile functions and the
//!   `PendingLocalEdit*` / `PendingScope` types. Its inner `ops` / `history`
//!   modules are **private**; consumers must not reach past the re-exports.
//! - `write_state` — the `WriteConfirmation` lifecycle type.
//!
//! Callers read their signal into plain data (`get_untracked()`) and pass it
//! in, so this band never depends back on `hooks/` (no `runtime -> hooks`
//! inversion). Per `00_engineering_constitution.md` §7, this typed surface is
//! the only sanctioned entry; the boundary is enforced by module privacy.
//! Decision context: `docs/report/web-runtime-band-convergence-decision-2026-05-29.md`.

pub mod confirm;
pub mod pending;
pub mod write_state;
