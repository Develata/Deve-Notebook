use super::*;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn p2p_status_initializes_without_token_material() {
    let _guard = TEST_LOCK.lock().expect("p2p status test lock");
    let peer = P2pPeerConfig {
        label: "peer-b".into(),
        peer_id: "peer-b".into(),
        repo_id: "11111111-1111-1111-1111-111111111111".into(),
        ws_url: "ws://127.0.0.1:3102/ws".into(),
        auth_token_env: "AUTH_SECRET_ENV".into(),
        enabled: true,
    };
    initialize(&P2pConfig {
        enabled: true,
        inbound_token_env: Some("SECRET_ENV".into()),
        connect_interval_ms: 1000,
        peers: vec![peer.clone()],
    });
    record_attempt(&peer);
    record_failure(&peer, "reconnecting", "connect_failed");

    let payload = node_role::get_node_role().p2p;
    assert!(payload.enabled);
    assert_eq!(payload.peers[0].label, "peer-b");
    assert_eq!(
        payload.peers[0].last_error_code.as_deref(),
        Some("connect_failed")
    );
    assert!(!format!("{payload:?}").contains("SECRET_ENV"));
}

#[test]
fn p2p_status_marks_peers_disabled_when_mesh_disabled() {
    let _guard = TEST_LOCK.lock().expect("p2p status test lock");
    let peer = P2pPeerConfig {
        label: "peer-b".into(),
        peer_id: "peer-b".into(),
        repo_id: "11111111-1111-1111-1111-111111111111".into(),
        ws_url: "ws://127.0.0.1:3102/ws".into(),
        auth_token_env: "AUTH_SECRET_ENV".into(),
        enabled: true,
    };
    initialize(&P2pConfig {
        enabled: false,
        inbound_token_env: Some("SECRET_ENV".into()),
        connect_interval_ms: 1000,
        peers: vec![peer],
    });

    let payload = node_role::get_node_role().p2p;
    assert!(!payload.enabled);
    assert_eq!(payload.peers[0].state, "disabled");
}

#[test]
fn p2p_status_duplicate_labels_do_not_share_state() {
    let _guard = TEST_LOCK.lock().expect("p2p status test lock");
    let peer_a = P2pPeerConfig {
        label: "edge".into(),
        peer_id: "peer-a".into(),
        repo_id: "11111111-1111-1111-1111-111111111111".into(),
        ws_url: "ws://127.0.0.1:3101/ws".into(),
        auth_token_env: "PEER_A_TOKEN".into(),
        enabled: true,
    };
    let peer_b = P2pPeerConfig {
        label: "edge".into(),
        peer_id: "peer-b".into(),
        repo_id: "22222222-2222-2222-2222-222222222222".into(),
        ws_url: "ws://127.0.0.1:3102/ws".into(),
        auth_token_env: "PEER_B_TOKEN".into(),
        enabled: true,
    };
    initialize(&P2pConfig {
        enabled: true,
        inbound_token_env: Some("SECRET_ENV".into()),
        connect_interval_ms: 1000,
        peers: vec![peer_a, peer_b.clone()],
    });

    record_failure(&peer_b, "error", "peer_id_mismatch");

    let payload = node_role::get_node_role().p2p;
    assert_eq!(payload.peers.len(), 2);
    assert_eq!(
        payload.peers[0].last_error_code.as_deref(),
        None,
        "first peer must not receive the second peer's state update"
    );
    assert_eq!(
        payload.peers[1].last_error_code.as_deref(),
        Some("peer_id_mismatch")
    );
}

#[test]
fn p2p_status_retry_preserves_last_error_until_success() {
    let _guard = TEST_LOCK.lock().expect("p2p status test lock");
    let peer = P2pPeerConfig {
        label: "peer-b".into(),
        peer_id: "peer-b".into(),
        repo_id: "11111111-1111-1111-1111-111111111111".into(),
        ws_url: "ws://127.0.0.1:3102/ws".into(),
        auth_token_env: "AUTH_SECRET_ENV".into(),
        enabled: true,
    };
    initialize(&P2pConfig {
        enabled: true,
        inbound_token_env: Some("SECRET_ENV".into()),
        connect_interval_ms: 1000,
        peers: vec![peer.clone()],
    });

    record_attempt(&peer);
    record_failure(&peer, "reconnecting", "connect_failed");
    record_attempt(&peer);

    let payload = node_role::get_node_role().p2p;
    assert_eq!(payload.peers[0].state, "connecting");
    assert_eq!(payload.peers[0].attempts, 2);
    assert_eq!(
        payload.peers[0].last_error_code.as_deref(),
        Some("connect_failed")
    );

    record_success(
        &peer,
        P2pExchangeOutcome {
            sent_pushes: 0,
            sent_snapshots: 0,
            applied_pushes: 0,
            applied_snapshots: 0,
        },
    );

    let payload = node_role::get_node_role().p2p;
    assert_eq!(payload.peers[0].state, "connected");
    assert_eq!(payload.peers[0].handshakes, 1);
    assert_eq!(payload.peers[0].last_error_code.as_deref(), None);
}
