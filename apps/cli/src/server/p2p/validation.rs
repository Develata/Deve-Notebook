//! plan_ref:
//!   - 07_network#full-peer-mesh-v1

use crate::server::p2p::stats::ExchangeStats;
use anyhow::{Context, Result, anyhow};
use deve_core::config::P2pPeerConfig;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::{
    DirectSyncPushAttributionInput, DirectSyncSnapshotAttributionInput, SourceProofRequirement,
    SyncPushHeader, SyncSourceProof, validate_direct_sync_push_attribution,
    validate_direct_sync_snapshot_attribution,
};
use deve_core::security::EncryptedOp;

pub(super) fn validate_authenticated_frame(
    peer: &P2pPeerConfig,
    expected_repo_id: RepoId,
    stats: &ExchangeStats,
    frame_repo_id: RepoId,
) -> Result<()> {
    validate_authenticated_exchange(peer, stats)?;
    if frame_repo_id != expected_repo_id {
        return Err(anyhow!(
            "P2P peer {} sent repo {} after handshake for configured repo {}",
            peer.label,
            frame_repo_id,
            expected_repo_id
        ));
    }
    Ok(())
}

pub(super) fn validate_authenticated_exchange<'a>(
    peer: &P2pPeerConfig,
    stats: &'a ExchangeStats,
) -> Result<&'a PeerId> {
    if !stats.saw_hello {
        return Err(anyhow!(
            "P2P peer {} sent sync payload before SyncHello",
            peer.label
        ));
    }
    stats.authenticated_peer_id.as_ref().ok_or_else(|| {
        anyhow!(
            "P2P peer {} sent sync payload before authenticated SyncHello",
            peer.label
        )
    })
}

pub(super) fn validate_requested_sources<'a>(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    sources: impl IntoIterator<Item = &'a PeerId>,
) -> Result<()> {
    validate_authenticated_exchange(peer, stats)?;
    for source in sources {
        if !stats.allowed_export_sources.contains(source) {
            return Err(anyhow!(
                "P2P request source {} was not offered to peer {} for repo {}",
                source,
                peer.label,
                repo_id
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_inbound_push(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    target_peer: &PeerId,
    source_peer_id: &PeerId,
    header: &SyncPushHeader,
    payload: &[EncryptedOp],
) -> Result<()> {
    let authenticated_peer =
        validate_requested_import_source(peer, repo_id, stats, source_peer_id)?;
    validate_direct_sync_push_attribution(DirectSyncPushAttributionInput {
        expected_repo_id: repo_id,
        authenticated_peer,
        declared_source_peer: source_peer_id,
        target_peer,
        header,
        payload,
        source_proof_requirement: SourceProofRequirement::Always,
    })
    .with_context(|| {
        format!(
            "P2P SyncPush source attribution rejected (source proof rejected) for peer {} repo {}",
            peer.label, repo_id
        )
    })?;
    Ok(())
}

pub(super) struct InboundSnapshotValidation<'a> {
    pub(super) target_peer: &'a PeerId,
    pub(super) source_peer_id: &'a PeerId,
    pub(super) server_vector: &'a VersionVector,
    pub(super) source_proof: Option<&'a SyncSourceProof>,
    pub(super) payload: &'a [EncryptedOp],
}

pub(super) fn validate_inbound_snapshot(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    input: InboundSnapshotValidation<'_>,
) -> Result<()> {
    let authenticated_peer =
        validate_requested_import_source(peer, repo_id, stats, input.source_peer_id)?;
    validate_direct_sync_snapshot_attribution(DirectSyncSnapshotAttributionInput {
        expected_repo_id: repo_id,
        authenticated_peer,
        declared_source_peer: input.source_peer_id,
        target_peer: input.target_peer,
        server_vector: input.server_vector,
        source_proof: input.source_proof,
        payload: input.payload,
        source_proof_requirement: SourceProofRequirement::Always,
    })
    .with_context(|| {
        format!(
            "P2P SyncPushSnapshot source attribution rejected (source proof rejected) for peer {} repo {}",
            peer.label, repo_id
        )
    })?;
    Ok(())
}

fn validate_requested_import_source<'a>(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &'a ExchangeStats,
    source_peer_id: &PeerId,
) -> Result<&'a PeerId> {
    let authenticated_peer = validate_authenticated_exchange(peer, stats)?;
    if !stats.requested_import_sources.contains(source_peer_id) {
        return Err(anyhow!(
            "P2P inbound source {} was not requested from peer {} for repo {}",
            source_peer_id,
            peer.label,
            repo_id
        ));
    }
    Ok(authenticated_peer)
}
