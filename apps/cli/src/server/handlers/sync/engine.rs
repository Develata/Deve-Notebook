//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use crate::server::{AppState, channel::DualChannel};
use deve_core::{models::RepoId, sync::engine::SyncEngine};
use std::sync::Arc;

use super::errors;

pub(super) fn with_strict<F, R>(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    scope_nonce: Option<u64>,
    f: F,
) -> Option<R>
where
    F: FnOnce(&SyncEngine) -> R,
{
    match state.sync_engine.with_strict_engine(repo_id, f) {
        Ok(result) => Some(result),
        Err(err) => {
            errors::sync_engine_failed(
                ch,
                format!("Failed to get sync engine for repo {}: {}", repo_id, err),
                scope_nonce,
            );
            None
        }
    }
}

pub(super) fn with_strict_mut<F, R>(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    scope_nonce: Option<u64>,
    f: F,
) -> Option<R>
where
    F: FnOnce(&mut SyncEngine) -> R,
{
    match state.sync_engine.with_strict_engine_mut(repo_id, f) {
        Ok(result) => Some(result),
        Err(err) => {
            errors::sync_engine_failed(
                ch,
                format!("Failed to get sync engine for repo {}: {}", repo_id, err),
                scope_nonce,
            );
            None
        }
    }
}
