//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#unauthorized-handling
//!   - 09_web_thin_client_ledger#web-edit-intent

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerErrorCode {
    #[serde(rename = "REQUEST_FAILED")]
    RequestFailed,
    #[serde(rename = "AUTH_TOKEN_EXPIRED")]
    AuthTokenExpired,
    #[serde(rename = "AUTH_TOKEN_MISSING")]
    AuthTokenMissing,
    #[serde(rename = "SYNC_EDIT_REJECTED")]
    SyncEditRejected,
    #[serde(rename = "SYNC_REPO_UNBOUND")]
    SyncRepoUnbound,
    #[serde(rename = "SYNC_PEER_UNAUTHENTICATED")]
    SyncPeerUnauthenticated,
    #[serde(rename = "SYNC_DECRYPT_FAILED")]
    SyncDecryptFailed,
    #[serde(rename = "SC_REPO_NOT_SELECTED")]
    ScRepoNotSelected,
    #[serde(rename = "SC_REMOTE_BRANCH_READONLY")]
    ScRemoteBranchReadonly,
    #[serde(rename = "SC_REPO_CONTEXT_INVALID")]
    ScRepoContextInvalid,
    #[serde(rename = "SC_PENDING_NOT_FOUND")]
    ScPendingNotFound,
    #[serde(rename = "SC_STAGED_NOT_FOUND")]
    ScStagedNotFound,
    #[serde(rename = "SC_DOC_NOT_FOUND")]
    ScDocNotFound,
    #[serde(rename = "SC_COMMIT_NOT_FOUND")]
    ScCommitNotFound,
    #[serde(rename = "SC_NOTHING_TO_COMMIT")]
    ScNothingToCommit,
    #[serde(rename = "SC_CONFLICT_TARGET_MISSING")]
    ScConflictTargetMissing,
    #[serde(rename = "STORAGE_DB_LOCKED")]
    StorageDbLocked,
    #[serde(rename = "STORAGE_NOT_FOUND")]
    StorageNotFound,
    #[serde(rename = "STORAGE_CONFLICT")]
    StorageConflict,
    #[serde(rename = "STORAGE_PERSIST_FAILED")]
    StoragePersistFailed,
    #[serde(rename = "PLUGIN_INVALID_MESSAGE")]
    PluginInvalidMessage,
    #[serde(rename = "PLUGIN_UNSUPPORTED_MESSAGE")]
    PluginUnsupportedMessage,
    #[serde(rename = "SC_COMMIT_DIFF_UNPROJECTABLE")]
    ScCommitDiffUnprojectable,
    #[serde(rename = "GRAPH_DEGRADED_PROJECTION_REQUIRED")]
    GraphDegradedProjectionRequired,
    #[serde(rename = "AUTH_INVALID_PASSWORD")]
    AuthInvalidPassword,
    #[serde(rename = "AUTH_RATE_LIMITED")]
    AuthRateLimited,
    #[serde(rename = "AUTH_CSRF_MISMATCH")]
    AuthCsrfMismatch,
    #[serde(rename = "SC_STALE_SCOPE")]
    ScStaleScope,
    #[serde(rename = "DOC_NOT_FOUND")]
    DocNotFound,
    #[serde(rename = "DOC_CONTEXT_INVALID")]
    DocContextInvalid,
    #[serde(rename = "SYNC_REPO_ROUTE_MISMATCH")]
    SyncRepoRouteMismatch,
    #[serde(rename = "SYNC_SNAPSHOT_REQUIRED")]
    SyncSnapshotRequired,
    #[serde(rename = "SYNC_INVALID_PAYLOAD")]
    SyncInvalidPayload,
    #[serde(rename = "SYNC_PEER_UNKNOWN")]
    SyncPeerUnknown,
    #[serde(rename = "SYNC_VERSION_MISMATCH")]
    SyncVersionMismatch,
    #[serde(rename = "SYNC_DISCONNECTED")]
    SyncDisconnected,
    #[serde(rename = "PLUGIN_UNKNOWN_PLUGIN")]
    PluginUnknownPlugin,
    #[serde(rename = "PLUGIN_CAPABILITY_DENIED")]
    PluginCapabilityDenied,
    #[serde(rename = "PLUGIN_RUNTIME_ERROR")]
    PluginRuntimeError,
    #[serde(rename = "PLUGIN_SERIALIZATION_ERROR")]
    PluginSerializationError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerError {
    pub code: ServerErrorCode,
    pub detail: Option<String>,
}

impl ServerError {
    pub const fn new(code: ServerErrorCode) -> Self {
        Self { code, detail: None }
    }

    pub fn with_detail(code: ServerErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: Some(detail.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerError, ServerErrorCode};
    use crate::codec;

    #[test]
    fn binary_codec_roundtrip_preserves_none_detail() {
        let encoded = codec::encode(&ServerError::new(
            ServerErrorCode::ScCommitDiffUnprojectable,
        ))
        .unwrap();
        let decoded: ServerError = codec::decode(&encoded).unwrap();
        assert_eq!(decoded.code, ServerErrorCode::ScCommitDiffUnprojectable);
        assert_eq!(decoded.detail, None);
    }

    #[test]
    fn binary_codec_roundtrip_preserves_some_detail() {
        let encoded = codec::encode(&ServerError::with_detail(
            ServerErrorCode::ScDocNotFound,
            "missing doc",
        ))
        .unwrap();
        let decoded: ServerError = codec::decode(&encoded).unwrap();
        assert_eq!(decoded.code, ServerErrorCode::ScDocNotFound);
        assert_eq!(decoded.detail.as_deref(), Some("missing doc"));
    }

    #[test]
    fn serde_name_for_graph_degraded_projection_required_is_stable() {
        let encoded = serde_json::to_string(&ServerError::new(
            ServerErrorCode::GraphDegradedProjectionRequired,
        ))
        .unwrap();
        assert!(encoded.contains("GRAPH_DEGRADED_PROJECTION_REQUIRED"));
    }

    #[test]
    fn serde_names_for_plan_error_catalog_codes_are_stable() {
        let cases = [
            (
                ServerErrorCode::AuthInvalidPassword,
                "AUTH_INVALID_PASSWORD",
            ),
            (ServerErrorCode::AuthRateLimited, "AUTH_RATE_LIMITED"),
            (ServerErrorCode::AuthCsrfMismatch, "AUTH_CSRF_MISMATCH"),
            (ServerErrorCode::ScStaleScope, "SC_STALE_SCOPE"),
            (ServerErrorCode::DocNotFound, "DOC_NOT_FOUND"),
            (ServerErrorCode::DocContextInvalid, "DOC_CONTEXT_INVALID"),
            (
                ServerErrorCode::SyncRepoRouteMismatch,
                "SYNC_REPO_ROUTE_MISMATCH",
            ),
            (
                ServerErrorCode::SyncSnapshotRequired,
                "SYNC_SNAPSHOT_REQUIRED",
            ),
            (ServerErrorCode::SyncInvalidPayload, "SYNC_INVALID_PAYLOAD"),
            (ServerErrorCode::SyncPeerUnknown, "SYNC_PEER_UNKNOWN"),
            (
                ServerErrorCode::SyncVersionMismatch,
                "SYNC_VERSION_MISMATCH",
            ),
            (ServerErrorCode::SyncDisconnected, "SYNC_DISCONNECTED"),
            (
                ServerErrorCode::PluginUnknownPlugin,
                "PLUGIN_UNKNOWN_PLUGIN",
            ),
            (
                ServerErrorCode::PluginCapabilityDenied,
                "PLUGIN_CAPABILITY_DENIED",
            ),
            (ServerErrorCode::PluginRuntimeError, "PLUGIN_RUNTIME_ERROR"),
            (
                ServerErrorCode::PluginSerializationError,
                "PLUGIN_SERIALIZATION_ERROR",
            ),
        ];

        for (code, expected) in cases {
            let encoded = serde_json::to_string(&ServerError::new(code)).unwrap();
            assert!(
                encoded.contains(expected),
                "encoded {code:?} as {encoded}, expected {expected}"
            );
        }
    }
}
