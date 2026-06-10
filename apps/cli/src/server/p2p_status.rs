//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 18_release#runtime-observability
//!
//! FullPeer mesh connector status published through `/api/node/role`.

use crate::server::node_role::{self, P2pPeerSummary, P2pSummary};
use deve_core::config::{P2pConfig, P2pPeerConfig};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Debug)]
pub(crate) struct P2pExchangeOutcome {
    pub sent_pushes: u64,
    pub sent_snapshots: u64,
    pub applied_pushes: u64,
    pub applied_snapshots: u64,
}

#[derive(Clone, Debug)]
struct P2pStatusState {
    enabled: bool,
    peers: Vec<P2pPeerStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct P2pPeerKey {
    peer_id: String,
    repo_id: String,
    ws_url: String,
}

#[derive(Clone, Debug)]
struct P2pPeerStatus {
    key: P2pPeerKey,
    summary: P2pPeerSummary,
}

static P2P_STATUS: OnceLock<Arc<RwLock<P2pStatusState>>> = OnceLock::new();

pub(crate) fn initialize(config: &P2pConfig) {
    let peers = config
        .peers
        .iter()
        .map(summary_from_config)
        .collect::<Vec<_>>();
    replace(P2pStatusState {
        enabled: config.enabled,
        peers,
    });
}

pub(crate) fn record_attempt(peer: &P2pPeerConfig) -> u64 {
    update_peer(peer, |peer| {
        peer.attempts = peer.attempts.saturating_add(1);
        peer.state = "connecting".into();
        peer.attempts
    })
    .unwrap_or(0)
}

pub(crate) fn record_success(peer: &P2pPeerConfig, outcome: P2pExchangeOutcome) {
    update_peer(peer, |peer| {
        peer.state = "connected".into();
        peer.handshakes = peer.handshakes.saturating_add(1);
        peer.sent_pushes = peer.sent_pushes.saturating_add(outcome.sent_pushes);
        peer.sent_snapshots = peer.sent_snapshots.saturating_add(outcome.sent_snapshots);
        peer.applied_pushes = peer.applied_pushes.saturating_add(outcome.applied_pushes);
        peer.applied_snapshots = peer
            .applied_snapshots
            .saturating_add(outcome.applied_snapshots);
        peer.last_error_code = None;
    });
}

pub(crate) fn record_failure(peer: &P2pPeerConfig, state: &str, error_code: &str) {
    update_peer(peer, |peer| {
        peer.state = state.into();
        peer.last_error_code = Some(error_code.into());
    });
}

fn summary_from_config(peer: &P2pPeerConfig) -> P2pPeerStatus {
    P2pPeerStatus {
        key: key_from_config(peer),
        summary: P2pPeerSummary {
            label: peer.label.clone(),
            peer_id: peer.peer_id.clone(),
            repo_id: peer.repo_id.clone(),
            state: if peer.enabled {
                "configured".into()
            } else {
                "disabled".into()
            },
            attempts: 0,
            handshakes: 0,
            sent_pushes: 0,
            sent_snapshots: 0,
            applied_pushes: 0,
            applied_snapshots: 0,
            last_error_code: None,
        },
    }
}

fn key_from_config(peer: &P2pPeerConfig) -> P2pPeerKey {
    P2pPeerKey {
        peer_id: peer.peer_id.clone(),
        repo_id: peer.repo_id.clone(),
        ws_url: peer.ws_url.clone(),
    }
}

fn replace(state: P2pStatusState) {
    let summary = summary_from_state(&state);
    match status_cell().write() {
        Ok(mut current) => *current = state,
        Err(_) => tracing::warn!("P2P status lock poisoned, ignoring replace"),
    }
    node_role::update_p2p_summary(summary);
}

fn update_peer<T>(peer: &P2pPeerConfig, apply: impl FnOnce(&mut P2pPeerSummary) -> T) -> Option<T> {
    let key = key_from_config(peer);
    let cell = status_cell();
    let (result, summary) = match cell.write() {
        Ok(mut current) => {
            let result = current
                .peers
                .iter_mut()
                .find(|status| status.key == key)
                .map(|status| apply(&mut status.summary));
            (result, summary_from_state(&current))
        }
        Err(_) => {
            tracing::warn!("P2P status lock poisoned, ignoring peer update");
            return None;
        }
    };
    node_role::update_p2p_summary(summary);
    result
}

fn summary_from_state(state: &P2pStatusState) -> P2pSummary {
    P2pSummary {
        enabled: state.enabled,
        peers: state
            .peers
            .iter()
            .map(|peer| peer.summary.clone())
            .collect(),
    }
}

fn status_cell() -> Arc<RwLock<P2pStatusState>> {
    P2P_STATUS
        .get_or_init(|| {
            Arc::new(RwLock::new(P2pStatusState {
                enabled: false,
                peers: Vec::new(),
            }))
        })
        .clone()
}

#[cfg(test)]
mod tests {
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
}
