//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!
//! Branch backup binding validation.
//!
//! This module only validates branch/writer to remote path metadata. It does
//! not bind credentials, open remote providers, upload packs, import restore
//! candidates, write ledger state, or touch Projection Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupBindingAccess {
    Writable,
    RemoteReadonly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchBindingInput {
    pub repo_id: RepoId,
    pub branch_name: String,
    pub writer_identity: String,
    pub local_writer_identity: String,
    pub branch_path: String,
    pub requested_access: BackupBindingAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchBinding {
    pub repo_id: RepoId,
    pub branch_name: String,
    pub writer_identity: String,
    pub branch_path: String,
    pub access: BackupBindingAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupBindingError {
    #[error("backup branch name is not a safe binding key: {0}")]
    UnsafeBranchName(String),
    #[error("non-local backup writer must be remote-readonly")]
    NonLocalWriterMustBeReadonly,
    #[error("backup branch/writer binding is duplicated")]
    DuplicateBranchWriterBinding,
    #[error("backup branch already has a writable binding")]
    DuplicateWritableBranch,
    #[error("backup branch path already has an active writer")]
    DuplicateActiveWriterPath,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn plan_backup_branch_binding(
    input: BackupBranchBindingInput,
) -> Result<BackupBranchBinding, BackupBindingError> {
    let branch_name = safe_branch_name(&input.branch_name)?;
    let writer_identity = safe_writer_identity(&input.writer_identity)?;
    let local_writer_identity = safe_writer_identity(&input.local_writer_identity)?;
    let branch_path = normalize_remote_path(&input.branch_path)?;

    if input.requested_access == BackupBindingAccess::Writable
        && writer_identity != local_writer_identity
    {
        return Err(BackupBindingError::NonLocalWriterMustBeReadonly);
    }

    Ok(BackupBranchBinding {
        repo_id: input.repo_id,
        branch_name,
        writer_identity,
        branch_path,
        access: input.requested_access,
    })
}

pub fn validate_backup_branch_bindings(
    bindings: &[BackupBranchBinding],
) -> Result<(), BackupBindingError> {
    let mut by_branch_writer = HashMap::new();
    let mut writable_by_branch = HashMap::new();
    let mut active_writer_by_path = HashMap::new();

    for binding in bindings {
        let branch_name = safe_branch_name(&binding.branch_name)?;
        let writer_identity = safe_writer_identity(&binding.writer_identity)?;
        let branch_path = normalize_remote_path(&binding.branch_path)?;

        let binding_key = (
            binding.repo_id,
            branch_name.clone(),
            writer_identity.clone(),
        );
        if by_branch_writer.insert(binding_key, ()).is_some() {
            return Err(BackupBindingError::DuplicateBranchWriterBinding);
        }

        if binding.access != BackupBindingAccess::Writable {
            continue;
        }

        let branch_key = (binding.repo_id, branch_name);
        if writable_by_branch
            .insert(branch_key, writer_identity.clone())
            .is_some()
        {
            return Err(BackupBindingError::DuplicateWritableBranch);
        }

        let path_key = (binding.repo_id, branch_path);
        if active_writer_by_path
            .insert(path_key, writer_identity)
            .is_some()
        {
            return Err(BackupBindingError::DuplicateActiveWriterPath);
        }
    }
    Ok(())
}

fn safe_branch_name(input: &str) -> Result<String, BackupBindingError> {
    if input.trim() != input || input.is_empty() || input == "." || input == ".." {
        return Err(BackupBindingError::UnsafeBranchName(input.to_string()));
    }
    if input
        .chars()
        .any(|ch| ch.is_ascii_control() || matches!(ch, '/' | '\\' | '\0'))
    {
        return Err(BackupBindingError::UnsafeBranchName(input.to_string()));
    }
    Ok(input.to_string())
}
