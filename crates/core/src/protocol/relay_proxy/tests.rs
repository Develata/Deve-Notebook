use super::{
    DirectSyncPushAttributionInput, DirectSyncSnapshotAttributionInput, RelayProxyRouteError,
    RelayProxyRouteInput, RelayProxySnapshotRouteInput, SourceProofRequirement,
    SyncPushAttributionInput, SyncSnapshotAttributionInput, plan_relay_proxy_route,
    plan_relay_proxy_snapshot_route, validate_direct_sync_push_attribution,
    validate_direct_sync_snapshot_attribution, validate_sync_push_attribution,
    validate_sync_snapshot_attribution,
};
use crate::models::{PeerId, RepoId, VersionVector};
use crate::protocol::{SyncPayloadKind, SyncPushHeader, SyncSourceProof};
use crate::security::{EncryptedOp, IdentityKeyPair};

fn repo_id() -> RepoId {
    uuid::Uuid::from_u128(11)
}

fn header(source_peer: &str) -> SyncPushHeader {
    SyncPushHeader {
        repo_id: repo_id(),
        peer_id: PeerId::new(source_peer),
        vector: VersionVector::new(),
        payload_kind: SyncPayloadKind::Diff,
        source_proof: None,
    }
}

fn source_proof() -> SyncSourceProof {
    SyncSourceProof {
        source_pubkey: vec![1; 32],
        payload_digest: vec![2; 32],
        signature: vec![3; 64],
    }
}

fn payload() -> Vec<EncryptedOp> {
    vec![EncryptedOp {
        doc_id: None,
        peer_seq: 1.into(),
        ciphertext: vec![1, 2, 3],
        nonce: vec![0; 12],
    }]
}

fn target_peer() -> PeerId {
    PeerId::new("target-peer")
}

fn input() -> RelayProxyRouteInput {
    RelayProxyRouteInput {
        expected_repo_id: repo_id(),
        authenticated_transport_peer: PeerId::new("source-peer"),
        declared_source_peer: PeerId::new("source-peer"),
        target_peer: PeerId::new("target-peer"),
        expected_payload_kind: SyncPayloadKind::Diff,
        header: header("source-peer"),
    }
}

fn snapshot_input() -> RelayProxySnapshotRouteInput {
    RelayProxySnapshotRouteInput {
        expected_repo_id: repo_id(),
        authenticated_transport_peer: PeerId::new("source-peer"),
        declared_source_peer: PeerId::new("source-peer"),
        target_peer: PeerId::new("target-peer"),
        source_proof_present: false,
    }
}

#[test]
fn direct_route_uses_header_repo_and_source_peer() {
    let route = plan_relay_proxy_route(input()).unwrap();

    assert_eq!(route.repo_id, repo_id());
    assert_eq!(route.transport_peer, PeerId::new("source-peer"));
    assert_eq!(route.source_peer, PeerId::new("source-peer"));
    assert_eq!(route.target_peer, PeerId::new("target-peer"));
    assert_eq!(route.payload_kind, SyncPayloadKind::Diff);
    assert!(!route.indirect_transport);
}

#[test]
fn indirect_route_preserves_source_attribution() {
    let mut input = input();
    input.authenticated_transport_peer = PeerId::new("relay-peer");
    input.header.source_proof = Some(source_proof());

    let route = plan_relay_proxy_route(input).unwrap();

    assert_eq!(route.transport_peer, PeerId::new("relay-peer"));
    assert_eq!(route.source_peer, PeerId::new("source-peer"));
    assert!(route.indirect_transport);
}

#[test]
fn indirect_route_requires_source_proof_presence() {
    let mut input = input();
    input.authenticated_transport_peer = PeerId::new("relay-peer");

    assert!(matches!(
        plan_relay_proxy_route(input),
        Err(RelayProxyRouteError::MissingSourceAttributionProof)
    ));
}

#[test]
fn rejects_repo_route_mismatch() {
    let mut input = input();
    input.expected_repo_id = uuid::Uuid::from_u128(12);

    assert!(matches!(
        plan_relay_proxy_route(input),
        Err(RelayProxyRouteError::RepoRouteMismatch)
    ));
}

#[test]
fn rejects_declared_source_or_payload_kind_mismatch() {
    let mut route_input = input();
    route_input.declared_source_peer = PeerId::new("other-source");
    assert!(matches!(
        plan_relay_proxy_route(route_input),
        Err(RelayProxyRouteError::SourceAttributionMismatch)
    ));

    let mut route_input = input();
    route_input.expected_payload_kind = SyncPayloadKind::Snapshot;
    assert!(matches!(
        plan_relay_proxy_route(route_input),
        Err(RelayProxyRouteError::PayloadKindMismatch)
    ));
}

#[test]
fn snapshot_route_uses_declared_source_without_fake_header() {
    let route = plan_relay_proxy_snapshot_route(snapshot_input()).unwrap();

    assert_eq!(route.repo_id, repo_id());
    assert_eq!(route.source_peer, PeerId::new("source-peer"));
    assert_eq!(route.payload_kind, SyncPayloadKind::Snapshot);
    assert!(!route.indirect_transport);
}

#[test]
fn indirect_snapshot_route_requires_source_proof_presence() {
    let mut route_input = snapshot_input();
    route_input.authenticated_transport_peer = PeerId::new("relay-peer");

    assert!(matches!(
        plan_relay_proxy_snapshot_route(route_input),
        Err(RelayProxyRouteError::MissingSourceAttributionProof)
    ));

    let mut route_input = snapshot_input();
    route_input.authenticated_transport_peer = PeerId::new("relay-peer");
    route_input.source_proof_present = true;
    let route = plan_relay_proxy_snapshot_route(route_input).unwrap();
    assert_eq!(route.transport_peer, PeerId::new("relay-peer"));
    assert_eq!(route.source_peer, PeerId::new("source-peer"));
    assert!(route.indirect_transport);
}

#[test]
fn rejects_empty_route_peers() {
    let mut route_input = input();
    route_input.header.peer_id = PeerId::new(" ");
    assert!(matches!(
        plan_relay_proxy_route(route_input),
        Err(RelayProxyRouteError::InvalidPeerId {
            field: "header.peer_id"
        })
    ));

    let mut route_input = input();
    route_input.target_peer = PeerId::new("");
    assert!(matches!(
        plan_relay_proxy_route(route_input),
        Err(RelayProxyRouteError::InvalidPeerId {
            field: "target_peer"
        })
    ));
}

#[test]
fn shared_push_attribution_accepts_direct_without_source_proof() {
    let source = PeerId::new("source-peer");
    let target = target_peer();
    let header = header(source.as_str());
    let payload = payload();

    let route = validate_sync_push_attribution(SyncPushAttributionInput {
        expected_repo_id: repo_id(),
        authenticated_transport_peer: &source,
        declared_source_peer: &source,
        target_peer: &target,
        header: &header,
        payload: &payload,
        source_proof_requirement: SourceProofRequirement::IndirectOnly,
    })
    .unwrap();

    assert!(!route.indirect_transport);
}

#[test]
fn shared_push_attribution_rejects_direct_source_mismatch() {
    let authenticated = PeerId::new("transport-peer");
    let source = PeerId::new("source-peer");
    let target = target_peer();
    let header = header(source.as_str());
    let payload = payload();

    let err = validate_direct_sync_push_attribution(DirectSyncPushAttributionInput {
        expected_repo_id: repo_id(),
        authenticated_peer: &authenticated,
        declared_source_peer: &source,
        target_peer: &target,
        header: &header,
        payload: &payload,
        source_proof_requirement: SourceProofRequirement::IndirectOnly,
    })
    .expect_err("direct FullPeer route must reject forged source");

    assert!(err.to_string().contains("source attribution"));
}

#[test]
fn shared_snapshot_attribution_requires_proof_when_policy_is_always() {
    let source = PeerId::new("source-peer");
    let target = target_peer();
    let vector = VersionVector::new();
    let payload = payload();

    let err = validate_direct_sync_snapshot_attribution(DirectSyncSnapshotAttributionInput {
        expected_repo_id: repo_id(),
        authenticated_peer: &source,
        declared_source_peer: &source,
        target_peer: &target,
        server_vector: &vector,
        source_proof: None,
        payload: &payload,
        source_proof_requirement: SourceProofRequirement::Always,
    })
    .expect_err("static FullPeer snapshot push must require proof");

    assert!(err.to_string().contains("source proof"));
}

#[test]
fn shared_snapshot_attribution_allows_direct_without_proof_when_policy_is_indirect_only() {
    let source = PeerId::new("source-peer");
    let target = target_peer();
    let vector = VersionVector::new();
    let payload = payload();

    let route = validate_sync_snapshot_attribution(SyncSnapshotAttributionInput {
        expected_repo_id: repo_id(),
        authenticated_transport_peer: &source,
        declared_source_peer: &source,
        target_peer: &target,
        server_vector: &vector,
        source_proof: None,
        payload: &payload,
        source_proof_requirement: SourceProofRequirement::IndirectOnly,
    })
    .unwrap();

    assert!(!route.indirect_transport);
}

#[test]
fn shared_snapshot_attribution_accepts_valid_required_proof() {
    let identity = IdentityKeyPair::generate();
    let source = identity.peer_id();
    let target = target_peer();
    let vector = VersionVector::new();
    let payload = payload();
    let proof = SyncSourceProof::sign(
        repo_id(),
        &source,
        &vector,
        SyncPayloadKind::Snapshot,
        &payload,
        &identity,
    )
    .unwrap();

    let route = validate_direct_sync_snapshot_attribution(DirectSyncSnapshotAttributionInput {
        expected_repo_id: repo_id(),
        authenticated_peer: &source,
        declared_source_peer: &source,
        target_peer: &target,
        server_vector: &vector,
        source_proof: Some(&proof),
        payload: &payload,
        source_proof_requirement: SourceProofRequirement::Always,
    })
    .unwrap();

    assert!(!route.indirect_transport);
}
