//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Manual merge shared helpers.

use crate::server::{AppState, channel::DualChannel};
use deve_core::config::SyncMode;
use deve_core::models::RepoId;
use deve_core::sync::engine::SyncEngine;
use std::sync::Arc;

use super::errors;

pub(super) fn sync_mode_label(mode: SyncMode) -> String {
    if matches!(mode, SyncMode::Auto) {
        "auto"
    } else {
        "manual"
    }
    .to_string()
}

pub(super) fn with_engine<F, R>(
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
            errors::classified_failure(
                ch,
                format!("Failed to get sync engine for repo {}: {}", repo_id, err),
                scope_nonce,
            );
            None
        }
    }
}

pub(super) fn with_engine_mut<F, R>(
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
            errors::classified_failure(
                ch,
                format!("Failed to get sync engine for repo {}: {}", repo_id, err),
                scope_nonce,
            );
            None
        }
    }
}
