//! Document runtime band — open / pending / ack-reject / navigation for the
//! thin-client write path.
//!
//! Phase B convergence target per `docs/tasks/19_repo_refactor_blueprint.md`
//! §3.3 and `docs/report/runtime-convergence-audit-2026-05-28.md`. The pending
//! overlay now lives here (`pending`, migrated out of `hooks/use_core/pending`
//! in Phase B step 2); history resend and the per-edit write-confirmation logic
//! still scattered across `hooks/use_core/callbacks_sync/write` and
//! `editor/sync` converge here onto the typed contracts below.

pub mod confirm;
pub mod pending;
pub mod write_state;
