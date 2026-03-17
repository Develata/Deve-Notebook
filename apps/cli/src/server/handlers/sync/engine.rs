use crate::server::{AppState, channel::DualChannel};
use deve_core::{models::RepoId, sync::engine::SyncEngine};
use std::sync::Arc;

use super::errors;

pub(super) fn load_strict(
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
