use super::*;
use deve_core::protocol::{ServerError, ServerErrorCode};

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
fn p2p_connector_retry_backoff_starts_at_one_second() {
    let mut backoff = INITIAL_RECONNECT_BACKOFF;
    assert_eq!(backoff, Duration::from_secs(1));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(2));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(4));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(8));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(16));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(30));
    backoff = next_backoff(backoff);
    assert_eq!(backoff, Duration::from_secs(30));
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
    let err = anyhow::anyhow!("HTTP 401 Unauthorized").context("Failed to connect P2P peer peer-b");

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
fn p2p_connector_static_token_header_errors_are_terminal() {
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "Invalid P2P bearer header for peer peer-b"
        )),
        "token_invalid"
    );
    assert!(is_terminal_p2p_error("token_invalid"));
    assert_eq!(failure_state("token_invalid"), "error");
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
fn p2p_connector_unrequested_source_is_terminal() {
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P inbound source peer-a was not requested from peer peer-b for repo 11111111-1111-1111-1111-111111111111"
        )),
        "unrequested_source"
    );
    assert!(is_terminal_p2p_error("unrequested_source"));
    assert_eq!(failure_state("unrequested_source"), "error");
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
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P cannot sign non-local snapshot source peer-a for repo 11111111-1111-1111-1111-111111111111"
        )),
        "source_proof_rejected"
    );
    assert!(is_terminal_p2p_error("source_proof_rejected"));
    assert_eq!(failure_state("source_proof_rejected"), "error");
}

#[test]
fn p2p_connector_duplicate_sync_hello_is_terminal() {
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer peer-b sent duplicate SyncHello during one exchange"
        )),
        "duplicate_sync_hello"
    );
    assert!(is_terminal_p2p_error("duplicate_sync_hello"));
    assert_eq!(failure_state("duplicate_sync_hello"), "error");
}

#[test]
fn p2p_connector_classifies_structured_protocol_errors() {
    let repo_error = ServerError::new(ServerErrorCode::SyncRepoRouteMismatch);
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer returned protocol error: {:?}",
            repo_error
        )),
        "repo_mismatch"
    );

    let source_error =
        ServerError::with_detail(ServerErrorCode::SyncInvalidPayload, "source_proof_rejected");
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer returned protocol error: {:?}",
            source_error
        )),
        "source_proof_rejected"
    );

    let peer_auth_error =
        ServerError::with_detail(ServerErrorCode::SyncPeerUnauthenticated, "Handshake failed");
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer returned protocol error: {:?}",
            peer_auth_error
        )),
        "malformed_session_proof"
    );

    let unoffered_source_error =
        ServerError::with_detail(ServerErrorCode::SyncPeerUnauthenticated, "unoffered_source");
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer returned protocol error: {:?}",
            unoffered_source_error
        )),
        "unoffered_source"
    );

    let unrequested_source_error = ServerError::with_detail(
        ServerErrorCode::SyncPeerUnauthenticated,
        "unrequested_source",
    );
    assert_eq!(
        classify_p2p_error(&anyhow::anyhow!(
            "P2P peer returned protocol error: {:?}",
            unrequested_source_error
        )),
        "unrequested_source"
    );
}
