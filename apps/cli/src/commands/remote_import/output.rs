//! plan_ref:
//!   - 06_backup#projection-backup-command-output-contract

use crate::local_cli_proxy_contract::LocalCliRemoteImportResponse;
use anyhow::{Result, anyhow};
use deve_core::protocol::{RemoteImportResponse, ServerErrorCode};

pub(super) fn print(response: &LocalCliRemoteImportResponse) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(response)?);
    Ok(())
}

pub(super) fn ensure_success(response: &LocalCliRemoteImportResponse) -> Result<()> {
    let code = match response {
        LocalCliRemoteImportResponse::Intent {
            response: RemoteImportResponse::Error { error, .. },
        }
        | LocalCliRemoteImportResponse::Error { error, .. } => Some(error.code),
        _ => None,
    };
    match code {
        Some(code) => Err(anyhow!(code_name(code))),
        None => Ok(()),
    }
}

pub(super) fn code_name(code: ServerErrorCode) -> &'static str {
    match code {
        ServerErrorCode::RemoteProjectionLocatorInvalid => "REMOTE_PROJECTION_LOCATOR_INVALID",
        ServerErrorCode::RemoteProjectionProviderUnavailable => {
            "REMOTE_PROJECTION_PROVIDER_UNAVAILABLE"
        }
        ServerErrorCode::RemoteImportActiveSession => "REMOTE_IMPORT_ACTIVE_SESSION",
        ServerErrorCode::RemoteImportNotFound => "REMOTE_IMPORT_NOT_FOUND",
        ServerErrorCode::RemoteImportStale => "REMOTE_IMPORT_STALE",
        ServerErrorCode::RemoteImportBlocked => "REMOTE_IMPORT_BLOCKED",
        ServerErrorCode::RemoteImportInvalidState => "REMOTE_IMPORT_INVALID_STATE",
        ServerErrorCode::RemoteImportLimitExceeded => "REMOTE_IMPORT_LIMIT_EXCEEDED",
        ServerErrorCode::RemoteImportPrepareFailed => "REMOTE_IMPORT_PREPARE_FAILED",
        ServerErrorCode::RemoteImportApplyFailed => "REMOTE_IMPORT_APPLY_FAILED",
        ServerErrorCode::RemoteImportCleanupRequired => "REMOTE_IMPORT_CLEANUP_REQUIRED",
        ServerErrorCode::StorageWorkspaceIngestionUnavailable => {
            "STORAGE_WORKSPACE_INGESTION_UNAVAILABLE"
        }
        _ => "REMOTE_IMPORT_INVALID_STATE",
    }
}
