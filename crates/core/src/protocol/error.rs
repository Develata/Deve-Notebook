//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 09_auth#unauthorized-handling
//!   - 16_web_thin_client_ledger#web-edit-intent

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

    #[test]
    fn bincode_roundtrip_preserves_none_detail() {
        let encoded = bincode::serialize(&ServerError::new(
            ServerErrorCode::ScCommitDiffUnprojectable,
        ))
        .unwrap();
        let decoded: ServerError = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.code, ServerErrorCode::ScCommitDiffUnprojectable);
        assert_eq!(decoded.detail, None);
    }

    #[test]
    fn bincode_roundtrip_preserves_some_detail() {
        let encoded = bincode::serialize(&ServerError::with_detail(
            ServerErrorCode::ScDocNotFound,
            "missing doc",
        ))
        .unwrap();
        let decoded: ServerError = bincode::deserialize(&encoded).unwrap();
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
}
