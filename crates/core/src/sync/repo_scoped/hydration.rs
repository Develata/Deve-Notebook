//! plan_ref:
//!   - 05_network#server-ws-runtime

use crate::ledger::listing::RepoListing;
use crate::ledger::{RepoManager, range};
use crate::models::{PeerId, RepoId};
use crate::sync::vector::VersionVector;
use anyhow::{Result, anyhow};

pub(super) fn build_version_vector(
    repo: &RepoManager,
    local_peer_id: &PeerId,
    repo_id: RepoId,
) -> Result<VersionVector> {
    let mut vector = VersionVector::new();
    let local_seq = repo.run_on_local_repo_id(&repo_id, range::get_max_seq)?;
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
