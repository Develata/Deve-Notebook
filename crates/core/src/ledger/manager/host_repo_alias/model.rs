//! plan_ref:
//!   - 04_repository#host-repo-alias-contract

use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(super) const EXPORT_FORMAT: &str = "deve.host-repo-aliases";
pub(super) const EXPORT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRepoAliasBinding {
    pub repo_id: RepoId,
    pub alias: String,
    pub alias_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRepoAliasSetResult {
    pub binding: HostRepoAliasBinding,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRepoAliasImportSummary {
    pub accepted: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub warnings: Vec<HostRepoAliasImportWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRepoAliasImportWarning {
    pub index: usize,
    pub repo_id: Option<RepoId>,
    pub reason: HostRepoAliasImportWarningReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostRepoAliasImportWarningReason {
    EntryNotObject,
    EntrySchemaInvalid,
    RepoIdMissing,
    RepoIdNotString,
    RepoIdInvalid,
    AliasMissing,
    AliasNotString,
    AliasEmpty,
    AliasTooLong,
    AliasContainsControl,
    DuplicateRepoId,
    UnknownLocalRepo,
    AdmissionFailed,
}

impl std::fmt::Display for HostRepoAliasImportWarningReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EntryNotObject => "entry is not an object",
            Self::EntrySchemaInvalid => "entry contains unsupported fields",
            Self::RepoIdMissing => "repo_id is missing",
            Self::RepoIdNotString => "repo_id is not a string",
            Self::RepoIdInvalid => "repo_id is not a valid UUID",
            Self::AliasMissing => "alias is missing",
            Self::AliasNotString => "alias is not a string",
            Self::AliasEmpty => "alias is empty after trimming",
            Self::AliasTooLong => "alias exceeds 256 UTF-8 bytes",
            Self::AliasContainsControl => "alias contains a control character",
            Self::DuplicateRepoId => "repo_id occurs more than once; all occurrences skipped",
            Self::UnknownLocalRepo => "repo_id is not an active local repository",
            Self::AdmissionFailed => "local repository admission failed",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HostRepoAliasValidationError {
    #[error("alias is empty after trim")]
    Empty,
    #[error("alias exceeds 256 UTF-8 bytes")]
    TooLong,
    #[error("alias contains a control character")]
    ContainsControl,
}

#[derive(Debug, Error)]
pub enum HostRepoAliasError {
    #[error("host repo alias I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("host repo alias JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("host repo alias runtime failed: {0}")]
    Runtime(#[from] anyhow::Error),
    #[error("host repo alias store is invalid: {0}")]
    StoreInvalid(String),
    #[error("unsupported alias import format: {0}")]
    UnsupportedFormat(String),
    #[error("unsupported alias import version: {0}")]
    UnsupportedVersion(u64),
    #[error("invalid alias import document: {0}")]
    InvalidDocument(&'static str),
    #[error("alias import exceeds {budget} budget: actual={actual}, limit={limit}")]
    BudgetExceeded {
        budget: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("invalid repo alias: {0}")]
    InvalidAlias(#[from] HostRepoAliasValidationError),
    #[error("local repository is not active: {0}")]
    UnknownLocalRepo(RepoId),
    #[error("repo alias revision conflict for {repo_id}: expected {expected}, current {current}")]
    RevisionConflict {
        repo_id: RepoId,
        expected: u64,
        current: u64,
    },
    #[error("repo alias revision overflow for {0}")]
    RevisionOverflow(RepoId),
}

#[derive(Debug, Serialize)]
pub(super) struct HostRepoAliasExportDocument {
    pub format: &'static str,
    pub version: u32,
    pub aliases: Vec<HostRepoAliasExportEntry>,
}

#[derive(Debug, Serialize)]
pub(super) struct HostRepoAliasExportEntry {
    pub repo_id: RepoId,
    pub alias: String,
}

pub(super) fn normalize_alias(alias: &str) -> Result<String, HostRepoAliasValidationError> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Err(HostRepoAliasValidationError::Empty);
    }
    if alias.len() > 256 {
        return Err(HostRepoAliasValidationError::TooLong);
    }
    if alias.chars().any(char::is_control) {
        return Err(HostRepoAliasValidationError::ContainsControl);
    }
    Ok(alias.to_owned())
}
