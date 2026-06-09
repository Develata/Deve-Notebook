//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!
//! Static FullPeer mesh connector loop and reconnection policy.

use crate::server::{AppState, p2p, p2p_status};
use anyhow::Error;
use deve_core::config::{P2pConfig, P2pPeerConfig};
use std::sync::Arc;
use tokio::time::Duration;

const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(30);

pub(super) fn spawn_mesh_connectors(config: P2pConfig, state: Arc<AppState>) {
    p2p_status::initialize(&config);
    if !config.enabled {
        return;
    }

    let interval = Duration::from_millis(config.connect_interval_ms.clamp(1_000, 30_000));
    for peer in config.peers.into_iter().filter(|peer| peer.enabled) {
        let state = state.clone();
        if is_self_loop(&peer, &state) {
            p2p_status::record_failure(&peer, "self_loop", "self_loop");
            tracing::warn!(
                peer_label = %peer.label,
                peer_id = %peer.peer_id,
                repo_id = %peer.repo_id,
                "P2P mesh connector rejected self-loop"
            );
            continue;
        }
        tokio::spawn(async move {
            let mut backoff = interval;
            loop {
                let attempt = p2p_status::record_attempt(&peer);
                match p2p::connect_peer_once(&peer, state.clone()).await {
                    Ok(stats) => {
                        p2p_status::record_success(&peer, outcome_from_stats(&stats));
                        backoff = interval;
                        tokio::time::sleep(interval).await;
                    }
                    Err(err) => {
                        let error_code = classify_p2p_error(&err);
                        let state_name = failure_state(error_code);
                        p2p_status::record_failure(&peer, state_name, error_code);
                        tracing::warn!(
                            peer_label = %peer.label,
                            peer_id = %peer.peer_id,
                            attempt,
                            error_code,
                            "P2P mesh connector attempt failed: {err}"
                        );
                        if is_terminal_p2p_error(error_code) {
                            break;
                        }
                        tokio::time::sleep(backoff + jitter_for_attempt(&peer, attempt)).await;
                        backoff = next_backoff(backoff);
                    }
                }
            }
        });
    }
}

fn is_self_loop(peer: &P2pPeerConfig, state: &Arc<AppState>) -> bool {
    peer.peer_id == state.identity_key.peer_id().as_str()
        && uuid::Uuid::parse_str(&peer.repo_id).is_ok_and(|repo_id| {
            state
                .repo
                .list_local_repo_names_for_execution()
                .is_ok_and(|names| {
                    names.into_iter().any(|name| {
                        state
                            .repo
                            .get_repo_info_for(None, Some(&name))
                            .ok()
                            .flatten()
                            .is_some_and(|info| info.uuid == repo_id)
                    })
                })
        })
}

fn outcome_from_stats(stats: &p2p::ExchangeStats) -> p2p_status::P2pExchangeOutcome {
    p2p_status::P2pExchangeOutcome {
        sent_pushes: stats.sent_pushes,
        sent_snapshots: stats.sent_snapshots,
        applied_pushes: stats.applied_pushes,
        applied_snapshots: stats.applied_snapshots,
    }
}

fn failure_state(error_code: &str) -> &'static str {
    if error_code == "unauthorized" {
        "unauthorized"
    } else if error_code == "self_loop" {
        "self_loop"
    } else if matches!(
        error_code,
        "connect_failed" | "handshake_timeout" | "protocol_error"
    ) {
        "reconnecting"
    } else {
        "error"
    }
}

fn is_terminal_p2p_error(error_code: &str) -> bool {
    matches!(
        error_code,
        "unauthorized"
            | "self_loop"
            | "repo_mismatch"
            | "peer_id_mismatch"
            | "token_missing"
            | "token_empty"
            | "invalid_url"
            | "invalid_repo_id"
    )
}

fn next_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(MAX_RECONNECT_BACKOFF)
}

fn jitter_for_attempt(peer: &P2pPeerConfig, attempt: u64) -> Duration {
    let label = peer.label.len() as u64;
    Duration::from_millis((attempt.saturating_mul(97) + label.saturating_mul(53)) % 250)
}

fn classify_p2p_error(err: &Error) -> &'static str {
    let message = err.to_string().to_ascii_lowercase();
    if message.contains("token env is missing") {
        "token_missing"
    } else if message.contains("self-loop") {
        "self_loop"
    } else if message.contains("token env is empty") {
        "token_empty"
    } else if message.contains("invalid p2p ws_url") {
        "invalid_url"
    } else if message.contains("invalid p2p repo_id") {
        "invalid_repo_id"
    } else if message.contains("configured peer_id") {
        "peer_id_mismatch"
    } else if message.contains("401")
        || message.contains("403")
        || message.contains("unauthorized")
        || message.contains("forbidden")
    {
        "unauthorized"
    } else if message.contains("repo") && message.contains("expected") {
        "repo_mismatch"
    } else if message.contains("timed out") {
        "handshake_timeout"
    } else if message.contains("decode") {
        "decode_failed"
    } else if message.contains("protocol error") {
        "protocol_error"
    } else if message.contains("apply") {
        "apply_failed"
    } else if message.contains("connect") {
        "connect_failed"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2p_connector_backoff_caps_at_thirty_seconds() {
        assert_eq!(
            next_backoff(Duration::from_secs(16)),
            Duration::from_secs(30)
        );
        assert_eq!(
            next_backoff(Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn p2p_connector_error_classifier_keeps_auth_separate() {
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("HTTP 401")),
            "unauthorized"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P handshake timed out")),
            "handshake_timeout"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P self-loop rejected after handshake")),
            "self_loop"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "authenticated peer_id actual did not match configured peer_id expected"
            )),
            "peer_id_mismatch"
        );
    }

    #[test]
    fn p2p_connector_identity_mismatch_is_terminal() {
        assert!(is_terminal_p2p_error("peer_id_mismatch"));
        assert!(is_terminal_p2p_error("repo_mismatch"));
        assert!(is_terminal_p2p_error("unauthorized"));
        assert!(is_terminal_p2p_error("self_loop"));
        assert!(!is_terminal_p2p_error("connect_failed"));
        assert_eq!(failure_state("peer_id_mismatch"), "error");
    }

    #[test]
    fn p2p_connector_static_config_errors_are_terminal() {
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P token env is missing for peer peer-b")),
            "token_missing"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P token env is empty for peer peer-b")),
            "token_empty"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("Invalid P2P ws_url for peer peer-b")),
            "invalid_url"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("Invalid P2P repo_id for peer peer-b")),
            "invalid_repo_id"
        );
        assert!(is_terminal_p2p_error("token_missing"));
        assert!(is_terminal_p2p_error("token_empty"));
        assert!(is_terminal_p2p_error("invalid_url"));
        assert!(is_terminal_p2p_error("invalid_repo_id"));
    }
}
