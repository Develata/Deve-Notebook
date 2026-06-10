//! plan_ref:
//!   - 07_network#relay-proxy-attribution-contract
//!   - 07_network#trust-boundary
//!
//! Relay/proxy route admission and source-attribution validation.
//!
//! Relay and proxy nodes route by plaintext sync headers only. This module
//! does not mutate headers, write ledger state, or apply remote shadows.

use crate::models::{PeerId, RepoId, VersionVector};
use crate::protocol::{SyncPayloadKind, SyncPushHeader, SyncSourceProof, SyncSourceProofError};
use crate::security::EncryptedOp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayProxyRouteInput {
    pub expected_repo_id: RepoId,
    pub authenticated_transport_peer: PeerId,
    pub declared_source_peer: PeerId,
    pub target_peer: PeerId,
    pub expected_payload_kind: SyncPayloadKind,
    pub header: SyncPushHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayProxySnapshotRouteInput {
    pub expected_repo_id: RepoId,
    pub authenticated_transport_peer: PeerId,
    pub declared_source_peer: PeerId,
    pub target_peer: PeerId,
    pub source_proof_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayProxyRoute {
    pub repo_id: RepoId,
    pub transport_peer: PeerId,
    pub source_peer: PeerId,
    pub target_peer: PeerId,
    pub payload_kind: SyncPayloadKind,
    pub indirect_transport: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RelayProxyRouteError {
    #[error("relay/proxy route repo id does not match the expected repo route")]
    RepoRouteMismatch,
    #[error("relay/proxy route source attribution does not match the plaintext header")]
    SourceAttributionMismatch,
    #[error("relay/proxy route payload kind does not match the message route")]
    PayloadKindMismatch,
    #[error("relay/proxy route peer id is empty or whitespace: {field}")]
    InvalidPeerId { field: &'static str },
    #[error("indirect relay/proxy route requires source attribution proof")]
    MissingSourceAttributionProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProofRequirement {
    IndirectOnly,
    Always,
}

impl SourceProofRequirement {
    const fn requires(self, indirect_transport: bool) -> bool {
        match self {
            Self::IndirectOnly => indirect_transport,
            Self::Always => true,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncAttributionError {
    #[error("{0}")]
    Route(#[from] RelayProxyRouteError),
    #[error("source proof rejected: {0}")]
    SourceProof(#[from] SyncSourceProofError),
    #[error(
        "source attribution rejected: direct source {declared_source_peer} does not match authenticated peer {authenticated_peer}"
    )]
    DirectSourceMismatch {
        declared_source_peer: PeerId,
        authenticated_peer: PeerId,
    },
}

pub struct SyncPushAttributionInput<'a> {
    pub expected_repo_id: RepoId,
    pub authenticated_transport_peer: &'a PeerId,
    pub declared_source_peer: &'a PeerId,
    pub target_peer: &'a PeerId,
    pub header: &'a SyncPushHeader,
    pub payload: &'a [EncryptedOp],
    pub source_proof_requirement: SourceProofRequirement,
}

pub struct SyncSnapshotAttributionInput<'a> {
    pub expected_repo_id: RepoId,
    pub authenticated_transport_peer: &'a PeerId,
    pub declared_source_peer: &'a PeerId,
    pub target_peer: &'a PeerId,
    pub server_vector: &'a VersionVector,
    pub source_proof: Option<&'a SyncSourceProof>,
    pub payload: &'a [EncryptedOp],
    pub source_proof_requirement: SourceProofRequirement,
}

pub struct DirectSyncPushAttributionInput<'a> {
    pub expected_repo_id: RepoId,
    pub authenticated_peer: &'a PeerId,
    pub declared_source_peer: &'a PeerId,
    pub target_peer: &'a PeerId,
    pub header: &'a SyncPushHeader,
    pub payload: &'a [EncryptedOp],
    pub source_proof_requirement: SourceProofRequirement,
}

pub struct DirectSyncSnapshotAttributionInput<'a> {
    pub expected_repo_id: RepoId,
    pub authenticated_peer: &'a PeerId,
    pub declared_source_peer: &'a PeerId,
    pub target_peer: &'a PeerId,
    pub server_vector: &'a VersionVector,
    pub source_proof: Option<&'a SyncSourceProof>,
    pub payload: &'a [EncryptedOp],
    pub source_proof_requirement: SourceProofRequirement,
}

pub fn plan_relay_proxy_route(
    input: RelayProxyRouteInput,
) -> Result<RelayProxyRoute, RelayProxyRouteError> {
    if input.header.repo_id != input.expected_repo_id {
        return Err(RelayProxyRouteError::RepoRouteMismatch);
    }
    validate_peer_id(
        &input.authenticated_transport_peer,
        "authenticated_transport_peer",
    )?;
    validate_peer_id(&input.declared_source_peer, "declared_source_peer")?;
    validate_peer_id(&input.target_peer, "target_peer")?;
    validate_peer_id(&input.header.peer_id, "header.peer_id")?;
    if input.header.peer_id != input.declared_source_peer {
        return Err(RelayProxyRouteError::SourceAttributionMismatch);
    }
    if input.header.payload_kind != input.expected_payload_kind {
        return Err(RelayProxyRouteError::PayloadKindMismatch);
    }

    let indirect_transport = input.authenticated_transport_peer != input.declared_source_peer;
    if indirect_transport && input.header.source_proof.is_none() {
        return Err(RelayProxyRouteError::MissingSourceAttributionProof);
    }

    Ok(RelayProxyRoute {
        repo_id: input.header.repo_id,
        transport_peer: input.authenticated_transport_peer,
        source_peer: input.declared_source_peer,
        target_peer: input.target_peer,
        payload_kind: input.header.payload_kind,
        indirect_transport,
    })
}

pub fn plan_relay_proxy_snapshot_route(
    input: RelayProxySnapshotRouteInput,
) -> Result<RelayProxyRoute, RelayProxyRouteError> {
    validate_peer_id(
        &input.authenticated_transport_peer,
        "authenticated_transport_peer",
    )?;
    validate_peer_id(&input.declared_source_peer, "declared_source_peer")?;
    validate_peer_id(&input.target_peer, "target_peer")?;

    let indirect_transport = input.authenticated_transport_peer != input.declared_source_peer;
    if indirect_transport && !input.source_proof_present {
        return Err(RelayProxyRouteError::MissingSourceAttributionProof);
    }

    Ok(RelayProxyRoute {
        repo_id: input.expected_repo_id,
        transport_peer: input.authenticated_transport_peer,
        source_peer: input.declared_source_peer,
        target_peer: input.target_peer,
        payload_kind: SyncPayloadKind::Snapshot,
        indirect_transport,
    })
}

fn validate_peer_id(peer_id: &PeerId, field: &'static str) -> Result<(), RelayProxyRouteError> {
    let value = peer_id.as_str();
    if value.trim().is_empty() {
        return Err(RelayProxyRouteError::InvalidPeerId { field });
    }
    Ok(())
}

pub fn validate_sync_push_attribution(
    input: SyncPushAttributionInput<'_>,
) -> Result<RelayProxyRoute, SyncAttributionError> {
    let route = plan_relay_proxy_route(RelayProxyRouteInput {
        expected_repo_id: input.expected_repo_id,
        authenticated_transport_peer: input.authenticated_transport_peer.clone(),
        declared_source_peer: input.declared_source_peer.clone(),
        target_peer: input.target_peer.clone(),
        expected_payload_kind: SyncPayloadKind::Diff,
        header: input.header.clone(),
    })?;
    input.header.validate_source_proof(
        input.payload,
        input
            .source_proof_requirement
            .requires(route.indirect_transport),
    )?;
    Ok(route)
}

pub fn validate_direct_sync_push_attribution(
    input: DirectSyncPushAttributionInput<'_>,
) -> Result<RelayProxyRoute, SyncAttributionError> {
    require_direct_source(input.authenticated_peer, input.declared_source_peer)?;
    validate_sync_push_attribution(SyncPushAttributionInput {
        expected_repo_id: input.expected_repo_id,
        authenticated_transport_peer: input.authenticated_peer,
        declared_source_peer: input.declared_source_peer,
        target_peer: input.target_peer,
        header: input.header,
        payload: input.payload,
        source_proof_requirement: input.source_proof_requirement,
    })
}

pub fn validate_sync_snapshot_attribution(
    input: SyncSnapshotAttributionInput<'_>,
) -> Result<RelayProxyRoute, SyncAttributionError> {
    let route = plan_relay_proxy_snapshot_route(RelayProxySnapshotRouteInput {
        expected_repo_id: input.expected_repo_id,
        authenticated_transport_peer: input.authenticated_transport_peer.clone(),
        declared_source_peer: input.declared_source_peer.clone(),
        target_peer: input.target_peer.clone(),
        source_proof_present: input.source_proof.is_some(),
    })?;
    validate_snapshot_source_proof(
        input.expected_repo_id,
        input.declared_source_peer,
        input.server_vector,
        input.source_proof,
        input.payload,
        input
            .source_proof_requirement
            .requires(route.indirect_transport),
    )?;
    Ok(route)
}

pub fn validate_direct_sync_snapshot_attribution(
    input: DirectSyncSnapshotAttributionInput<'_>,
) -> Result<RelayProxyRoute, SyncAttributionError> {
    require_direct_source(input.authenticated_peer, input.declared_source_peer)?;
    validate_sync_snapshot_attribution(SyncSnapshotAttributionInput {
        expected_repo_id: input.expected_repo_id,
        authenticated_transport_peer: input.authenticated_peer,
        declared_source_peer: input.declared_source_peer,
        target_peer: input.target_peer,
        server_vector: input.server_vector,
        source_proof: input.source_proof,
        payload: input.payload,
        source_proof_requirement: input.source_proof_requirement,
    })
}

fn require_direct_source(
    authenticated_peer: &PeerId,
    declared_source_peer: &PeerId,
) -> Result<(), SyncAttributionError> {
    if authenticated_peer != declared_source_peer {
        return Err(SyncAttributionError::DirectSourceMismatch {
            declared_source_peer: declared_source_peer.clone(),
            authenticated_peer: authenticated_peer.clone(),
        });
    }
    Ok(())
}

fn validate_snapshot_source_proof(
    repo_id: RepoId,
    peer_id: &PeerId,
    server_vector: &VersionVector,
    source_proof: Option<&SyncSourceProof>,
    payload: &[EncryptedOp],
    required: bool,
) -> Result<(), SyncSourceProofError> {
    match source_proof {
        Some(proof) => proof.verify(
            repo_id,
            peer_id,
            server_vector,
            SyncPayloadKind::Snapshot,
            payload,
        ),
        None if required => Err(SyncSourceProofError::Missing),
        None => Ok(()),
    }
}
