use super::{RelayProxyRouteError, RelayProxyRouteInput, plan_relay_proxy_route};
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
        target_peer: PeerId::new("target-peer"),
        header: header("source-peer"),
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
