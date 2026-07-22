//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#unauthorized-handling
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 13_i18n#i18n-error-code-catalog

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
    #[serde(rename = "STORAGE_WORKSPACE_INGESTION_UNAVAILABLE")]
    StorageWorkspaceIngestionUnavailable,
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
    #[serde(rename = "DIFF_RESOURCE_LIMIT")]
    DiffResourceLimit,
    #[serde(rename = "DIFF_COMPUTE_FAILED")]
    DiffComputeFailed,
    #[serde(rename = "REMOTE_PROJECTION_LOCATOR_INVALID")]
    RemoteProjectionLocatorInvalid,
    #[serde(rename = "REMOTE_PROJECTION_PROVIDER_UNAVAILABLE")]
    RemoteProjectionProviderUnavailable,
    #[serde(rename = "REMOTE_PROJECTION_PUSH_FAILED")]
    RemoteProjectionPushFailed,
    #[serde(rename = "REMOTE_IMPORT_ACTIVE_SESSION")]
    RemoteImportActiveSession,
    #[serde(rename = "REMOTE_IMPORT_NOT_FOUND")]
    RemoteImportNotFound,
    #[serde(rename = "REMOTE_IMPORT_STALE")]
    RemoteImportStale,
    #[serde(rename = "REMOTE_IMPORT_BLOCKED")]
    RemoteImportBlocked,
    #[serde(rename = "REMOTE_IMPORT_INVALID_STATE")]
    RemoteImportInvalidState,
    #[serde(rename = "REMOTE_IMPORT_LIMIT_EXCEEDED")]
    RemoteImportLimitExceeded,
    #[serde(rename = "REMOTE_IMPORT_PREPARE_FAILED")]
    RemoteImportPrepareFailed,
    #[serde(rename = "REMOTE_IMPORT_APPLY_FAILED")]
    RemoteImportApplyFailed,
    #[serde(rename = "REMOTE_IMPORT_CLEANUP_REQUIRED")]
    RemoteImportCleanupRequired,
    #[serde(rename = "REPO_ALIAS_INVALID")]
    RepoAliasInvalid,
    #[serde(rename = "REPO_ALIAS_STALE")]
    RepoAliasStale,
    #[serde(rename = "REPO_ALIAS_STORE_FAILED")]
    RepoAliasStoreFailed,
    #[serde(rename = "REPO_LIFECYCLE_BUSY")]
    RepoLifecycleBusy,
    #[serde(rename = "REPO_LIFECYCLE_NOT_FOUND")]
    RepoLifecycleNotFound,
    #[serde(rename = "REPO_LIFECYCLE_INVALID_REQUEST")]
    RepoLifecycleInvalidRequest,
    #[serde(rename = "REPO_LIFECYCLE_AUTHORITY_BUSY")]
    RepoLifecycleAuthorityBusy,
    #[serde(rename = "REPO_LIFECYCLE_REMOVAL_BLOCKED")]
    RepoLifecycleRemovalBlocked,
    #[serde(rename = "REPO_LIFECYCLE_CONFIRMATION_INVALID")]
    RepoLifecycleConfirmationInvalid,
    #[serde(rename = "REPO_LIFECYCLE_CONFIRMATION_EXPIRED")]
    RepoLifecycleConfirmationExpired,
    #[serde(rename = "REPO_LIFECYCLE_CONFIRMATION_STALE")]
    RepoLifecycleConfirmationStale,
    #[serde(rename = "REPO_LIFECYCLE_COMMITTED_PARTIAL")]
    RepoLifecycleCommittedPartial,
    #[serde(rename = "REPO_LIFECYCLE_REPAIR_REQUIRED")]
    RepoLifecycleRepairRequired,
    #[serde(rename = "REPO_LIFECYCLE_PUBLICATION_PENDING")]
    RepoLifecyclePublicationPending,
    #[serde(rename = "REPO_CREATION_PROJECTION_BASE_REQUIRED")]
    RepoCreationProjectionBaseRequired,
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

    pub fn workspace_ingestion_unavailable() -> Self {
        Self::with_detail(
            ServerErrorCode::StorageWorkspaceIngestionUnavailable,
            "Workspace changes are temporarily unavailable; restart the service to recover.",
        )
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
    fn workspace_ingestion_error_uses_fixed_generic_detail() {
        let error = ServerError::workspace_ingestion_unavailable();
        assert_eq!(
            error.code,
            ServerErrorCode::StorageWorkspaceIngestionUnavailable
        );
        assert_eq!(
            error.detail.as_deref(),
            Some("Workspace changes are temporarily unavailable; restart the service to recover.")
        );
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
            (ServerErrorCode::DiffResourceLimit, "DIFF_RESOURCE_LIMIT"),
            (ServerErrorCode::DiffComputeFailed, "DIFF_COMPUTE_FAILED"),
            (
                ServerErrorCode::StorageWorkspaceIngestionUnavailable,
                "STORAGE_WORKSPACE_INGESTION_UNAVAILABLE",
            ),
            (
                ServerErrorCode::RemoteProjectionLocatorInvalid,
                "REMOTE_PROJECTION_LOCATOR_INVALID",
            ),
            (
                ServerErrorCode::RemoteProjectionProviderUnavailable,
                "REMOTE_PROJECTION_PROVIDER_UNAVAILABLE",
            ),
            (
                ServerErrorCode::RemoteProjectionPushFailed,
                "REMOTE_PROJECTION_PUSH_FAILED",
            ),
            (
                ServerErrorCode::RemoteImportActiveSession,
                "REMOTE_IMPORT_ACTIVE_SESSION",
            ),
            (
                ServerErrorCode::RemoteImportNotFound,
                "REMOTE_IMPORT_NOT_FOUND",
            ),
            (ServerErrorCode::RemoteImportStale, "REMOTE_IMPORT_STALE"),
            (
                ServerErrorCode::RemoteImportBlocked,
                "REMOTE_IMPORT_BLOCKED",
            ),
            (
                ServerErrorCode::RemoteImportInvalidState,
                "REMOTE_IMPORT_INVALID_STATE",
            ),
            (
                ServerErrorCode::RemoteImportLimitExceeded,
                "REMOTE_IMPORT_LIMIT_EXCEEDED",
            ),
            (
                ServerErrorCode::RemoteImportPrepareFailed,
                "REMOTE_IMPORT_PREPARE_FAILED",
            ),
            (
                ServerErrorCode::RemoteImportApplyFailed,
                "REMOTE_IMPORT_APPLY_FAILED",
            ),
            (
                ServerErrorCode::RemoteImportCleanupRequired,
                "REMOTE_IMPORT_CLEANUP_REQUIRED",
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
