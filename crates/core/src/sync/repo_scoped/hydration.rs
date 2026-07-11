//! plan_ref:
//!   - 07_network#server-ws-runtime

use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::models::VersionVector;
use crate::models::{PeerId, RepoId};
use anyhow::{Result, anyhow};

pub(super) fn build_version_vector(
    repo: &RepoManager,
    local_peer_id: &PeerId,
    repo_id: RepoId,
) -> Result<VersionVector> {
    let mut vector = VersionVector::new();
    if repo.local_peer_id() != local_peer_id {
        return Err(anyhow!(
            "Sync engine local peer {} does not match repo host identity {}",
            local_peer_id,
            repo.local_peer_id()
        ));
    }
    let local_seq = repo.get_local_peer_waterline(&repo_id)?;
    vector.set_exact(local_peer_id.clone(), local_seq);

    for peer_id in repo.list_shadows_on_disk()? {
        if repo
            .resolve_remote_repo_entry_by_id(&peer_id, repo_id)?
            .is_none()
        {
            continue;
        }
        if peer_id == *local_peer_id {
            return Err(anyhow!(
                "Shadow peer {} collides with local peer for repo {}",
                peer_id,
                repo_id
            ));
        }
        let shadow_seq = repo.get_shadow_max_seq(&peer_id, &repo_id)?;
        vector.set_exact(peer_id, shadow_seq);
    }

    Ok(vector)
}
