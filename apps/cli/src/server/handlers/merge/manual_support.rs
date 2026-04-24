//! plan_ref:
//!   - 06_repository#repo-scope-runtime
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

pub(super) fn load_engine(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    scope_nonce: Option<u64>,
) -> Option<SyncEngine> {
    match state.sync_engine.get_or_create_strict(repo_id) {
        Ok(engine) => Some(engine),
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
