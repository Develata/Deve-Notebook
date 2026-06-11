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
            | "malformed_session_proof"
            | "unoffered_source"
            | "source_proof_rejected"
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
    let message = err
        .chain()
        .map(|cause| cause.to_string().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(": ");
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
    } else if message.contains("configured peer_id") || message.contains("peerid mismatch") {
        "peer_id_mismatch"
    } else if message.contains("invalid handshake signature")
        || message.contains("synchello proof rejected")
    {
        "malformed_session_proof"
    } else if message.contains("401")
        || message.contains("403")
        || message.contains("unauthorized")
        || message.contains("forbidden")
    {
        "unauthorized"
    } else if message.contains("repo") && message.contains("expected")
        || message.contains("sent repo") && message.contains("configured repo")
    {
        "repo_mismatch"
    } else if message.contains("request source") && message.contains("not offered") {
        "unoffered_source"
    } else if message.contains("source proof rejected")
        || message.contains("source attribution rejected")
    {
        "source_proof_rejected"
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
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P peer peer-b SyncHello proof rejected: Invalid Handshake Signature"
            )),
            "malformed_session_proof"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P peer peer-b SyncHello proof rejected: PeerID mismatch: claimed a, derived b"
            )),
            "peer_id_mismatch"
        );
    }

    #[test]
    fn p2p_connector_error_classifier_scans_error_chain() {
        let err =
            anyhow::anyhow!("HTTP 401 Unauthorized").context("Failed to connect P2P peer peer-b");

        assert_eq!(classify_p2p_error(&err), "unauthorized");
        assert!(is_terminal_p2p_error(classify_p2p_error(&err)));
    }

    #[test]
    fn p2p_connector_identity_mismatch_is_terminal() {
        assert!(is_terminal_p2p_error("peer_id_mismatch"));
        assert!(is_terminal_p2p_error("malformed_session_proof"));
        assert!(is_terminal_p2p_error("unauthorized"));
        assert!(is_terminal_p2p_error("self_loop"));
        assert!(is_terminal_p2p_error("unoffered_source"));
        assert!(is_terminal_p2p_error("source_proof_rejected"));
        assert!(!is_terminal_p2p_error("connect_failed"));
        assert_eq!(failure_state("peer_id_mismatch"), "error");
    }

    #[test]
    fn p2p_connector_repo_mismatch_is_terminal() {
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P peer peer-b sent repo 22222222-2222-2222-2222-222222222222 after handshake for configured repo 11111111-1111-1111-1111-111111111111"
            )),
            "repo_mismatch"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P peer peer-b repo route mismatch: expected 11111111-1111-1111-1111-111111111111"
            )),
            "repo_mismatch"
        );
        assert!(is_terminal_p2p_error("repo_mismatch"));
        assert_eq!(failure_state("repo_mismatch"), "error");
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

    #[test]
    fn p2p_connector_unoffered_source_is_terminal() {
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P request source peer-a was not offered to peer peer-b for repo 11111111-1111-1111-1111-111111111111"
            )),
            "unoffered_source"
        );
        assert!(is_terminal_p2p_error("unoffered_source"));
        assert_eq!(failure_state("unoffered_source"), "error");
    }

    #[test]
    fn p2p_connector_source_proof_rejection_is_terminal() {
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P SyncPush source proof rejected")),
            "source_proof_rejected"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!(
                "P2P SyncPushSnapshot source proof rejected"
            )),
            "source_proof_rejected"
        );
        assert_eq!(
            classify_p2p_error(&anyhow::anyhow!("P2P SyncPush source attribution rejected")),
            "source_proof_rejected"
        );
        assert!(is_terminal_p2p_error("source_proof_rejected"));
        assert_eq!(failure_state("source_proof_rejected"), "error");
    }
}
