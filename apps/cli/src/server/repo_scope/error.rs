//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Repo scope error taxonomy and protocol mapping.

use crate::server::error_classify::{
    is_db_locked, is_remote_exact_selector_mismatch, is_repo_context_invalid, is_repo_not_selected,
    is_stale_scope, is_storage_corruption, is_storage_not_found,
};
use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::fmt;

pub const STALE_REMOTE_SCOPE_PREFIX: &str = "stale remote scope:";

pub fn stale_remote_scope_detail(detail: impl AsRef<str>) -> String {
    format!("{STALE_REMOTE_SCOPE_PREFIX} {}", detail.as_ref())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoScopeFailure {
    RepoUnbound { detail: String },
    StaleScope { detail: String },
    RemoteBranchUnavailable { detail: String },
    ExactSelectorMismatch { detail: String },
    RepoContextInvalid { detail: String },
    StorageNotFound { detail: String },
    StorageDbLocked { detail: String },
    StoragePersistFailed { detail: String },
    RequestFailed { detail: String },
}

impl RepoScopeFailure {
    pub fn repo_unbound(detail: impl Into<String>) -> Self {
        Self::RepoUnbound {
            detail: detail.into(),
        }
    }

    pub fn stale_scope(detail: impl Into<String>) -> Self {
        Self::StaleScope {
            detail: detail.into(),
        }
    }

    pub fn remote_branch_unavailable(branch: impl fmt::Display) -> Self {
        Self::RemoteBranchUnavailable {
            detail: stale_remote_scope_detail(format!("Remote branch not available: {branch}")),
        }
    }

    pub fn exact_selector_mismatch(detail: impl Into<String>) -> Self {
        Self::ExactSelectorMismatch {
            detail: detail.into(),
        }
    }

    pub fn repo_context_invalid(detail: impl Into<String>) -> Self {
        Self::RepoContextInvalid {
            detail: detail.into(),
        }
    }

    pub fn storage_persist_failed(detail: impl Into<String>) -> Self {
        Self::StoragePersistFailed {
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> ServerErrorCode {
        match self {
            Self::RepoUnbound { .. } => ServerErrorCode::SyncRepoUnbound,
            Self::StaleScope { .. } | Self::RemoteBranchUnavailable { .. } => {
                ServerErrorCode::ScStaleScope
            }
            Self::ExactSelectorMismatch { .. } | Self::RepoContextInvalid { .. } => {
                ServerErrorCode::ScRepoContextInvalid
            }
            Self::StorageNotFound { .. } => ServerErrorCode::StorageNotFound,
            Self::StorageDbLocked { .. } => ServerErrorCode::StorageDbLocked,
            Self::StoragePersistFailed { .. } => ServerErrorCode::StoragePersistFailed,
            Self::RequestFailed { .. } => ServerErrorCode::RequestFailed,
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::RepoUnbound { detail }
            | Self::StaleScope { detail }
            | Self::RemoteBranchUnavailable { detail }
            | Self::ExactSelectorMismatch { detail }
            | Self::RepoContextInvalid { detail }
            | Self::StorageNotFound { detail }
            | Self::StorageDbLocked { detail }
            | Self::StoragePersistFailed { detail }
            | Self::RequestFailed { detail } => detail,
        }
    }

    pub fn is_remote_branch_unavailable(&self) -> bool {
        matches!(self, Self::RemoteBranchUnavailable { .. })
    }

    pub fn to_server_error(&self) -> ServerError {
        ServerError::with_detail(self.code(), self.detail())
    }

    pub fn from_anyhow(error: &Error) -> Option<&Self> {
        error.downcast_ref::<Self>()
    }
}

impl fmt::Display for RepoScopeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.detail())
    }
}

impl std::error::Error for RepoScopeFailure {}

impl From<RepoScopeFailure> for ServerError {
    fn from(error: RepoScopeFailure) -> Self {
        error.to_server_error()
    }
}

pub fn map_repo_scope_error(error: Error) -> ServerError {
    map_repo_scope_error_ref(&error)
}

pub fn map_repo_scope_error_ref(error: &Error) -> ServerError {
    if let Some(error) = RepoScopeFailure::from_anyhow(error) {
        return error.to_server_error();
    }
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return ServerError::with_detail(ServerErrorCode::SyncRepoUnbound, detail);
    }
    if is_storage_not_found(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, detail);
    }
    if is_db_locked(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageDbLocked, detail);
    }
    if is_storage_corruption(&lower) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, detail);
    }
    if is_remote_exact_selector_mismatch(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    if is_stale_scope(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScStaleScope, detail);
    }
    if is_repo_context_invalid(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

#[cfg(test)]
mod tests;
