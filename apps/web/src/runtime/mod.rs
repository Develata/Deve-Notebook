//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! Web client runtime bands.
//!
//! Infra-first runtime convergence (Phase B+) per
//! `docs/tasks/19_repo_refactor_blueprint.md` §3.3 and
//! `docs/report/runtime-convergence-audit-2026-05-28.md`: scattered runtime
//! logic under `hooks/use_core/` (the `effects_*` / `callbacks_*` prefix
//! families) is migrated here into `runtime/*_client` bands with typed APIs.
//! These modules are Flow Coordination / Object Plane adapters only; they never
//! own ledger, projection, or source-control authority.

pub mod document;
pub mod document_client;
pub mod external_changes_client;
pub mod rendering_client;
pub mod scope_client;
pub mod session_client;
pub mod source_control_client;

#[derive(Clone)]
pub struct CoreRuntimeClients {
    pub session: session_client::SessionClient,
    pub scope: scope_client::ScopeClient,
    pub document: document_client::DocumentClient,
    pub source_control: source_control_client::SourceControlClient,
    pub external_changes: external_changes_client::ExternalChangesClient,
    pub rendering: rendering_client::RenderingClient,
}
