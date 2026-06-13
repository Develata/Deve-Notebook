//! plan_ref:
//!   - 07_network#full-peer-mesh-v1

use crate::server::AppState;
use anyhow::{Context, Result};
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::sync::protocol as sync_proto;
use std::sync::Arc;

pub(super) struct SyncSourceSets {
    pub(super) allowed_export_sources: Vec<PeerId>,
    pub(super) requested_import_sources: Vec<PeerId>,
}

pub(super) fn sync_source_sets_for_hello(
    state: &Arc<AppState>,
    repo_id: RepoId,
    authenticated_peer: &PeerId,
    remote_vector: &VersionVector,
) -> Result<SyncSourceSets> {
    let local_peer = state.identity_key.peer_id();
    state
        .sync_engine
        .with_strict_engine(repo_id, |engine| {
            let (to_send, to_request, snapshot_requests) =
                sync_proto::compute_diff_requests(engine.version_vector(), remote_vector, repo_id);
            let mut allowed_export_sources = Vec::new();
            for request in to_send {
                if request.peer_id == local_peer
                    && !allowed_export_sources.contains(&request.peer_id)
                {
                    allowed_export_sources.push(request.peer_id);
                }
            }
            let mut requested_import_sources = Vec::new();
            for peer_id in to_request
                .into_iter()
                .map(|request| request.peer_id)
                .chain(snapshot_requests.into_iter().map(|request| request.peer_id))
            {
                if &peer_id == authenticated_peer && !requested_import_sources.contains(&peer_id) {
                    requested_import_sources.push(peer_id);
                }
            }
            SyncSourceSets {
                allowed_export_sources,
                requested_import_sources,
            }
        })
        .with_context(|| format!("Failed to compute P2P source sets for {repo_id}"))
}
