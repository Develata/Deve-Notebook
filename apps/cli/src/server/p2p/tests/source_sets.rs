use super::super::source_sets::sync_source_sets_for_hello;
use super::support::{
    REMOTE_PEER_ID, THIRD_PARTY_PEER_ID, append_local_op, append_remote_shadow_op,
    test_state_with_dir,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::security::IdentityKeyPair;
use std::sync::Arc;

#[test]
fn p2p_fullpeer_offer_set_excludes_third_party_shadow_sources() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let local_peer = identity.peer_id();
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    let third_party = PeerId::new(THIRD_PARTY_PEER_ID);

    append_local_op(&state, repo_id)?;
    append_remote_shadow_op(&state, repo_id, &third_party)?;

    let offered = sync_source_sets_for_hello(
        &state,
        repo_id,
        &PeerId::new(REMOTE_PEER_ID),
        &VersionVector::new(),
    )?
    .allowed_export_sources;

    assert!(offered.contains(&local_peer));
    assert!(
        !offered.contains(&third_party),
        "FullPeer v1 must not advertise shadow sources without retained origin proof"
    );
    Ok(())
}

#[test]
fn p2p_fullpeer_request_set_excludes_third_party_shadow_sources() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    state.sync_engine.get_or_create_strict(repo_id)?;
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let third_party = PeerId::new(THIRD_PARTY_PEER_ID);
    let mut remote_vector = VersionVector::new();
    remote_vector.update(authenticated_peer.clone(), 1);
    remote_vector.update(third_party.clone(), 1);

    let requested =
        sync_source_sets_for_hello(&state, repo_id, &authenticated_peer, &remote_vector)?
            .requested_import_sources;

    assert!(requested.contains(&authenticated_peer));
    assert!(
        !requested.contains(&third_party),
        "FullPeer v1 must not request third-party shadow sources from a direct peer"
    );
    Ok(())
}
