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
) -> Option<SyncEngine> {
    state.sync_engine.get_or_create(repo_id).or_else(|| {
        errors::request_failed(ch, "Failed to get or create sync engine");
        None
    })
}
