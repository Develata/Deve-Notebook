//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
mod dir_change;
#[cfg(not(target_arch = "wasm32"))]
mod dir_refresh_guard;
#[cfg(not(target_arch = "wasm32"))]
mod discard_pending;
#[cfg(not(target_arch = "wasm32"))]
pub mod drift_detect;
#[cfg(not(target_arch = "wasm32"))]
pub mod engine;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
#[cfg(not(target_arch = "wasm32"))]
mod manager_lifecycle;
#[cfg(not(target_arch = "wasm32"))]
mod manager_reconcile;
#[cfg(not(target_arch = "wasm32"))]
mod manager_workspace;
#[cfg(not(target_arch = "wasm32"))]
mod materialize;
#[cfg(not(target_arch = "wasm32"))]
mod pending;
#[cfg(not(target_arch = "wasm32"))]
mod pending_content;
#[cfg(not(target_arch = "wasm32"))]
mod pending_rename;
#[cfg(not(target_arch = "wasm32"))]
mod projection_diagnostic;
#[cfg(not(target_arch = "wasm32"))]
mod projection_health;
#[cfg(not(target_arch = "wasm32"))]
mod projection_io;
#[cfg(not(target_arch = "wasm32"))]
mod projection_persistence_runtime;
#[cfg(not(target_arch = "wasm32"))]
mod projection_plan;
#[cfg(not(target_arch = "wasm32"))]
mod projection_repair_runtime;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub mod rebuild;
#[cfg(not(target_arch = "wasm32"))]
mod rebuild_projection;
#[cfg(not(target_arch = "wasm32"))]
mod rebuild_projection_state;
#[cfg(not(target_arch = "wasm32"))]
pub mod reconcile;
#[cfg(not(target_arch = "wasm32"))]
pub mod repo_scoped;
#[cfg(not(target_arch = "wasm32"))]
pub mod scan;
#[cfg(not(target_arch = "wasm32"))]
mod scan_file;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot_policy;
pub mod vector;
#[cfg(not(target_arch = "wasm32"))]
pub mod watcher;

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::RepoManager;
#[cfg(not(target_arch = "wasm32"))]
use crate::vfs::Vfs;
#[cfg(not(target_arch = "wasm32"))]
use crate::writeback::PersistGuard;
#[cfg(not(target_arch = "wasm32"))]
use dir_refresh_guard::DirRefreshGuard;
#[cfg(not(target_arch = "wasm32"))]
pub use projection_diagnostic::{
    ProjectionDiagnostic, ProjectionDiagnosticIssue, ProjectionDiagnosticStatus,
};
#[cfg(not(target_arch = "wasm32"))]
use projection_health::ProjectionHealth;
#[cfg(not(target_arch = "wasm32"))]
pub use projection_repair_runtime::diagnose_projection_local_repo;
#[cfg(not(target_arch = "wasm32"))]
use snapshot_policy::SnapshotPolicy;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
pub struct SyncManager {
    repo: Arc<RepoManager>,
    vfs: Vfs,
    dir_refresh_guard: DirRefreshGuard,
    persist_guard: Arc<PersistGuard>,
    projection_health: ProjectionHealth,
}
