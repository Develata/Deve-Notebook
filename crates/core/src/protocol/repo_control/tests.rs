//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!
//! F4/v5 repo-control wire and opaque-capability tests.

use super::*;
use crate::protocol::frame::{
    ClientFrame, ServerFrame, WS_FRAME_MAGIC, WS_PROTOCOL_VERSION, decode_client_binary,
    decode_client_json, decode_server_binary, decode_server_json, encode_client_binary,
    encode_server_binary,
};
use crate::protocol::{ClientMessage, ServerMessage};

#[test]
fn opaque_removal_secrets_validate_shape_and_redact_debug() {
    let value = "a".repeat(64);
    let token = RemovalConfirmationToken::from_backend(value.clone()).expect("token");
    let fallback = OpaqueFallbackBinding::from_backend(value).expect("fallback");
    assert_eq!(format!("{token:?}"), "RemovalConfirmationToken([redacted])");
    assert_eq!(format!("{fallback:?}"), "OpaqueFallbackBinding([redacted])");
    assert!(RemovalConfirmationToken::from_backend("A".repeat(64)).is_none());
    assert!(OpaqueFallbackBinding::from_backend("a".repeat(63)).is_none());
    assert!(serde_json::from_str::<RemovalConfirmationToken>(r#""short""#).is_err());
    assert!(
        serde_json::from_str::<OpaqueFallbackBinding>(&format!("\"{}\"", "A".repeat(64))).is_err()
    );
}

#[test]
fn repo_control_nested_wire_roundtrips_in_f4_v5_binary_and_versioned_json() {
    let request_id = Uuid::from_u128(0x401);
    let preparation_id = Uuid::from_u128(0x402);
    let repo_id = Uuid::from_u128(0x403);
    let token = RemovalConfirmationToken::from_backend("a".repeat(64)).expect("token");
    let fallback_binding =
        OpaqueFallbackBinding::from_backend("b".repeat(64)).expect("fallback binding");
    let requests = [
        RepoControlRequest::PrepareLocalRepoRemoval {
            request_id,
            repo_id,
            current_scope_nonce: ScopeNonce::new(9),
            fallback_repo_id: Some(Uuid::from_u128(0x404)),
        },
        RepoControlRequest::ExecuteLocalRepoRemoval {
            request_id: Uuid::from_u128(0x405),
            preparation_id,
            confirmation_token: token.clone(),
            fallback_binding: Some(fallback_binding.clone()),
            current_scope_nonce: ScopeNonce::new(9),
            switch_nonce: SwitchNonce::new(10),
        },
    ];
    for request in requests {
        assert_client_message_roundtrips(ClientMessage::RepoControl(request));
    }
    assert_client_message_roundtrips(ClientMessage::RepoControl(
        RepoControlRequest::SubmitLifecycle {
            request_id: Uuid::from_u128(0x406),
            lifecycle_intent: RepoLifecycleIntent::Create {
                initial_alias: "created".into(),
                current_scope_nonce: ScopeNonce::new(9),
                switch_nonce: SwitchNonce::new(10),
            },
        },
    ));

    assert_server_message_roundtrips(ServerMessage::RepoControl(
        RepoControlResponse::LocalRepoRemovalPrepared {
            request_id,
            preparation_id,
            repo_id,
            preview: LocalRepoRemovalPreview {
                deleted: Vec::new(),
                preserved: vec![LocalRepoRemovalPreservedCategory::WorkspaceContent],
                warnings: vec![LocalRepoRemovalWarning::LedgerHistoryHasNoSupportedRestore],
                blockers: Vec::new(),
            },
            confirmation_token: Some(token),
            fallback_binding: Some(fallback_binding),
            expires_at_unix_ms: Some(1_800_000),
        },
    ));
    for scope in [
        RepoRemovalFinalScope::RepoBound {
            repo_id: Uuid::from_u128(0x407),
            scope_nonce: ScopeNonce::new(10),
        },
        RepoRemovalFinalScope::NoScope {
            scope_nonce: ScopeNonce::new(10),
        },
    ] {
        assert_server_message_roundtrips(ServerMessage::RepoControl(
            RepoControlResponse::LocalRepoRemovalSettled {
                request_id: Uuid::from_u128(0x408),
                job_id: Uuid::from_u128(0x409),
                removed_repo_id: repo_id,
                final_repo_list: Vec::new(),
                scope,
            },
        ));
    }
}

#[test]
fn direct_remove_lifecycle_intent_is_absent_from_f4_v5() {
    let legacy = serde_json::json!({
        "protocol_version": WS_PROTOCOL_VERSION,
        "message": {
            "RepoControl": {
                "SubmitLifecycle": {
                    "request_id": Uuid::from_u128(0x410),
                    "lifecycle_intent": {
                        "operation": "remove",
                        "repo_id": Uuid::from_u128(0x411),
                        "current_scope_nonce": 7,
                        "switch_nonce": 8
                    }
                }
            }
        }
    });
    assert!(decode_client_json(&legacy.to_string()).is_err());
}

#[test]
fn repo_removal_execute_wire_binds_distinct_request_and_preparation_ids() {
    let request_id = Uuid::from_u128(0x420);
    let preparation_id = Uuid::from_u128(0x421);
    let message = ClientMessage::RepoControl(RepoControlRequest::ExecuteLocalRepoRemoval {
        request_id,
        preparation_id,
        confirmation_token: RemovalConfirmationToken::from_backend("c".repeat(64)).expect("token"),
        fallback_binding: None,
        current_scope_nonce: ScopeNonce::new(11),
        switch_nonce: SwitchNonce::new(12),
    });
    let decoded = decode_client_binary(&encode_client_binary(&message).expect("encode"))
        .expect("decode exact execute identity");
    match decoded {
        ClientMessage::RepoControl(RepoControlRequest::ExecuteLocalRepoRemoval {
            request_id: actual_request_id,
            preparation_id: actual_preparation_id,
            ..
        }) => {
            assert_eq!(actual_request_id, request_id);
            assert_eq!(actual_preparation_id, preparation_id);
            assert_ne!(actual_request_id, actual_preparation_id);
        }
        other => panic!("unexpected decoded message: {other:?}"),
    }
}

#[test]
fn repo_removal_wire_rejects_malformed_opaque_confirmation_values() {
    let message = ClientMessage::RepoControl(RepoControlRequest::ExecuteLocalRepoRemoval {
        request_id: Uuid::from_u128(0x430),
        preparation_id: Uuid::from_u128(0x431),
        confirmation_token: RemovalConfirmationToken::from_backend("d".repeat(64)).expect("token"),
        fallback_binding: Some(
            OpaqueFallbackBinding::from_backend("e".repeat(64)).expect("fallback"),
        ),
        current_scope_nonce: ScopeNonce::new(13),
        switch_nonce: SwitchNonce::new(14),
    });

    let mut binary = encode_client_binary(&message).expect("encode valid removal execute");
    let token_offset = binary
        .windows(64)
        .position(|window| window == "d".repeat(64).as_bytes())
        .expect("encoded token bytes");
    binary[token_offset] = b'D';
    assert!(decode_client_binary(&binary).is_err());

    let mut json = serde_json::to_value(ClientFrame::current(message)).expect("encode JSON value");
    json["message"]["RepoControl"]["ExecuteLocalRepoRemoval"]["confirmation_token"] =
        serde_json::json!("short");
    assert!(decode_client_json(&json.to_string()).is_err());
}

fn assert_client_message_roundtrips(message: ClientMessage) {
    let expected = serde_json::to_value(&message).expect("serialize expected client message");
    let binary = encode_client_binary(&message).expect("encode F4/v5 client frame");
    assert!(binary.starts_with(WS_FRAME_MAGIC));
    assert_eq!(
        serde_json::to_value(decode_client_binary(&binary).expect("decode client message"))
            .expect("serialize decoded client message"),
        expected
    );
    let json = serde_json::to_string(&ClientFrame::current(message))
        .expect("encode versioned debug client JSON");
    assert_eq!(
        serde_json::to_value(decode_client_json(&json).expect("decode client debug JSON"))
            .expect("serialize decoded client JSON"),
        expected
    );
}

fn assert_server_message_roundtrips(message: ServerMessage) {
    let expected = serde_json::to_value(&message).expect("serialize expected server message");
    let binary = encode_server_binary(&message).expect("encode F4/v5 server frame");
    assert!(binary.starts_with(WS_FRAME_MAGIC));
    assert_eq!(
        serde_json::to_value(decode_server_binary(&binary).expect("decode server message"))
            .expect("serialize decoded server message"),
        expected
    );
    let json = serde_json::to_string(&ServerFrame::current(message))
        .expect("encode versioned debug server JSON");
    assert_eq!(
        serde_json::to_value(decode_server_json(&json).expect("decode server debug JSON"))
            .expect("serialize decoded server JSON"),
        expected
    );
}
