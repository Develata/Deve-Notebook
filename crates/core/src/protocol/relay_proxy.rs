//! plan_ref:
//!   - 05_network#relay-proxy-attribution-contract
//!
//! Relay/proxy route admission.
//!
//! Relay and proxy nodes route by plaintext sync headers only. This module
//! does not inspect encrypted payload bodies, validate source signatures,
//! mutate headers, write ledger state, or apply remote shadows.

use crate::models::{PeerId, RepoId};
use crate::protocol::{SyncPayloadKind, SyncPushHeader};
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
