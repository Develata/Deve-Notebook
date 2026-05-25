//! plan_ref:
//!   - 06_backup#backup-command-output-contract
//!
//! Backup command output model.
//!
//! This module normalizes command-visible backup status, plan, and error
//! categories. It does not replace module-specific validation errors, execute
//! commands, call providers, mutate bindings, append ledger entries, or touch
//! Projection Workspaces.

use super::binding::{BackupBindingAccess, BackupBranchBinding};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupCommandKind {
    BindBackupTarget,
    InspectBackupTarget,
    ListBackupBranches,
    BackupBranch,
    VerifyBackupTarget,
    RestoreBackup,
    UnbindBackupTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupBindingStatus {
    Unbound,
    Writable,
    RemoteReadonly,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupPlanEffect {
    InspectOnly,
    BindingMutation,
    RemoteUpload,
    RemoteVerify,
    RemoteDownload,
    ExplicitImport,
    ExplicitMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPlanInput {
    pub command: BackupCommandKind,
    pub binding_status: BackupBindingStatus,
    pub effect: BackupPlanEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPlan {
    pub command: BackupCommandKind,
    pub binding_status: BackupBindingStatus,
    pub effect: BackupPlanEffect,
    pub writes_local_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupErrorKind {
    LocatorUnreachable,
    CredentialRejected,
    ManifestMissingOrMalformed,
    RepoIdMismatch,
    DuplicateWritableBranchBinding,
    PackHashMismatch,
    AuthenticationFailed,
    DecryptFailure,
    RemoteVersionConflict,
    RestoreCandidateIncompatible,
    InvalidInput,
    ForbiddenEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupError {
    pub command: BackupCommandKind,
    pub kind: BackupErrorKind,
    pub fail_closed: bool,
    pub partial_effects_forbidden: bool,
}

impl BackupError {
    pub fn fail_closed(command: BackupCommandKind, kind: BackupErrorKind) -> Self {
        Self {
            command,
            kind,
            fail_closed: true,
            partial_effects_forbidden: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BackupCommandOutputError {
    #[error("backup command effect does not match the command")]
    CommandEffectMismatch,
    #[error("backup command requires a writable binding")]
    WritableBindingRequired,
    #[error("backup command cannot proceed while binding status is conflicted")]
    BindingConflict,
}

pub fn backup_binding_status(binding: Option<&BackupBranchBinding>) -> BackupBindingStatus {
    match binding.map(|binding| binding.access) {
        None => BackupBindingStatus::Unbound,
        Some(BackupBindingAccess::Writable) => BackupBindingStatus::Writable,
        Some(BackupBindingAccess::RemoteReadonly) => BackupBindingStatus::RemoteReadonly,
    }
}

pub fn backup_command_plan(input: BackupPlanInput) -> Result<BackupPlan, BackupCommandOutputError> {
    if input.binding_status == BackupBindingStatus::Conflict {
        return Err(BackupCommandOutputError::BindingConflict);
    }
    validate_command_effect(input.command, input.effect)?;
    if input.command == BackupCommandKind::BackupBranch
        && input.binding_status != BackupBindingStatus::Writable
    {
        return Err(BackupCommandOutputError::WritableBindingRequired);
    }

    Ok(BackupPlan {
        command: input.command,
        binding_status: input.binding_status,
        effect: input.effect,
        writes_local_authority: matches!(
            input.effect,
            BackupPlanEffect::ExplicitImport | BackupPlanEffect::ExplicitMerge
        ),
    })
}

fn validate_command_effect(
    command: BackupCommandKind,
    effect: BackupPlanEffect,
) -> Result<(), BackupCommandOutputError> {
    let ok = match command {
        BackupCommandKind::BindBackupTarget | BackupCommandKind::UnbindBackupTarget => {
            effect == BackupPlanEffect::BindingMutation
        }
        BackupCommandKind::InspectBackupTarget | BackupCommandKind::ListBackupBranches => {
            effect == BackupPlanEffect::InspectOnly
        }
        BackupCommandKind::BackupBranch => effect == BackupPlanEffect::RemoteUpload,
        BackupCommandKind::VerifyBackupTarget => effect == BackupPlanEffect::RemoteVerify,
        BackupCommandKind::RestoreBackup => matches!(
            effect,
            BackupPlanEffect::RemoteDownload
                | BackupPlanEffect::ExplicitImport
                | BackupPlanEffect::ExplicitMerge
        ),
    };
    if ok {
        Ok(())
    } else {
        Err(BackupCommandOutputError::CommandEffectMismatch)
    }
}
