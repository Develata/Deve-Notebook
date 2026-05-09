//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use crate::server::{AppState, repo_scope::ResolvedRepo};
use deve_core::models::PeerId;
use std::sync::Arc;
use tempfile::TempDir;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    super::super::super::repo_scope_recovery_support::build_state()
}

pub(super) fn seed_remote_shadow(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    repo_name: &str,
) -> anyhow::Result<()> {
    super::super::super::repo_scope_recovery_support::seed_remote_shadow(
        state, peer_id, repo_id, repo_name,
    )
}

pub(super) fn remote_scope(
    repo_id: uuid::Uuid,
    repo_name: impl Into<String>,
    peer_id: PeerId,
) -> ResolvedRepo {
    ResolvedRepo {
        repo_id,
        repo_name: repo_name.into(),
        branch: Some(peer_id),
    }
}
