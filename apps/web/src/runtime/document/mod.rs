//! Document runtime band — open / pending / ack-reject / navigation for the
//! thin-client write path.
//!
//! Phase B convergence target per `docs/tasks/19_repo_refactor_blueprint.md`
//! §3.3 and `docs/report/runtime-convergence-audit-2026-05-28.md`. The pending
//! overlay, history resend, and write-confirmation logic currently scattered
//! across `hooks/use_core/pending`, `hooks/use_core/callbacks_sync/write` and
//! `editor/sync` are migrated into this band onto the typed contract below.

pub mod write_state;
