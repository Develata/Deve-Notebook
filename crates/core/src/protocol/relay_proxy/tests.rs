use super::{
    RelayProxyRouteError, RelayProxyRouteInput, RelayProxySnapshotRouteInput,
    plan_relay_proxy_route, plan_relay_proxy_snapshot_route,
};
use crate::models::{PeerId, RepoId, VersionVector};
use crate::protocol::{SyncPayloadKind, SyncPushHeader, SyncSourceProof};

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
