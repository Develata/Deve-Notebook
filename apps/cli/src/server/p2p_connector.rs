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
const INITIAL_RECONNECT_BACKOFF: Duration = Duration::from_secs(1);

pub(super) fn spawn_mesh_connectors(
    config: P2pConfig,
    state: Arc<AppState>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Vec<tokio::task::JoinHandle<()>> {
    p2p_status::initialize(&config);
    if !config.enabled {
        return Vec::new();
    }

    let success_interval = Duration::from_millis(config.connect_interval_ms.clamp(1_000, 30_000));
    let mut tasks = Vec::new();
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
        let mut shutdown = shutdown.clone();
        tasks.push(tokio::spawn(async move {
            let mut backoff = INITIAL_RECONNECT_BACKOFF;
            loop {
                if *shutdown.borrow() {
                    break;
                }
                let attempt = p2p_status::record_attempt(&peer);
                tracing::debug!(
                    peer_label = %peer.label,
                    peer_id = %peer.peer_id,
                    repo_id = %peer.repo_id,
                    attempt,
                    "P2P mesh connector attempt started"
                );
                let exchange = tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                        continue;
                    }
                    result = p2p::connect_peer_once(&peer, state.clone()) => result,
                };
                match exchange {
                    Ok(stats) => {
                        p2p_status::record_success(&peer, outcome_from_stats(&stats));
                        backoff = INITIAL_RECONNECT_BACKOFF;
                        if sleep_or_shutdown(success_interval, &mut shutdown).await {
                            break;
                        }
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
                        if sleep_or_shutdown(
                            backoff + jitter_for_attempt(&peer, attempt),
                            &mut shutdown,
                        )
                        .await
                        {
                            break;
                        }
                        backoff = next_backoff(backoff);
                    }
                }
            }
        }));
    }
    tasks
}

async fn sleep_or_shutdown(
    duration: Duration,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    tokio::select! {
        changed = shutdown.changed() => changed.is_err() || *shutdown.borrow(),
        _ = tokio::time::sleep(duration) => false,
    }
}

fn is_self_loop(peer: &P2pPeerConfig, state: &Arc<AppState>) -> bool {
    peer.peer_id == state.identity_key.peer_id().as_str()
        && uuid::Uuid::parse_str(&peer.repo_id).is_ok_and(|repo_id| {
            state
                .repo
                .get_local_repo_info_by_id(repo_id)
                .is_ok_and(|info| info.is_some())
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
            | "unrequested_source"
            | "source_proof_rejected"
            | "duplicate_sync_hello"
            | "token_missing"
            | "token_empty"
            | "token_invalid"
            | "invalid_url"
            | "invalid_repo_id"
    )
}

fn next_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(MAX_RECONNECT_BACKOFF)
}

fn jitter_for_attempt(peer: &P2pPeerConfig, attempt: u64) -> Duration {
    let seed = peer_identity_jitter_seed(peer);
    Duration::from_millis((attempt.saturating_mul(97) + seed) % 250)
}

fn peer_identity_jitter_seed(peer: &P2pPeerConfig) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for part in [&peer.peer_id, &peer.repo_id, &peer.ws_url] {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn classify_p2p_error(err: &Error) -> &'static str {
    let message = err
        .chain()
        .map(|cause| cause.to_string().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(": ");
    let compact = message
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if message.contains("token env is missing") {
        "token_missing"
    } else if message.contains("self-loop") {
        "self_loop"
    } else if message.contains("token env is empty") {
        "token_empty"
    } else if message.contains("invalid p2p bearer header") {
        "token_invalid"
    } else if message.contains("invalid p2p ws_url") {
        "invalid_url"
    } else if message.contains("invalid p2p repo_id") {
        "invalid_repo_id"
    } else if message.contains("configured peer_id") || message.contains("peerid mismatch") {
        "peer_id_mismatch"
    } else if message.contains("request source") && message.contains("not offered")
        || compact.contains("unofferedsource")
    {
        "unoffered_source"
    } else if (message.contains("inbound source")
        || message.contains("syncpush source")
        || message.contains("syncpushsnapshot source"))
        && message.contains("not requested")
        || compact.contains("unrequestedsource")
    {
        "unrequested_source"
    } else if message.contains("source proof rejected")
        || message.contains("source attribution rejected")
        || message.contains("cannot sign non-local")
        || compact.contains("sourceproofrejected")
        || compact.contains("sourceattributionrejected")
    {
        "source_proof_rejected"
    } else if message.contains("401")
        || message.contains("403")
        || message.contains("unauthorized")
        || message.contains("forbidden")
    {
        "unauthorized"
    } else if message.contains("repo") && message.contains("expected")
        || message.contains("sent repo") && message.contains("configured repo")
        || compact.contains("syncreporoutemismatch")
    {
        "repo_mismatch"
    } else if compact.contains("duplicatesynchello") {
        "duplicate_sync_hello"
    } else if message.contains("invalid handshake signature")
        || message.contains("synchello proof rejected")
        || compact.contains("syncpeerunauthenticated")
    {
        "malformed_session_proof"
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
mod tests;
