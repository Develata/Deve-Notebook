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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerError {
    pub code: ServerErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
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
