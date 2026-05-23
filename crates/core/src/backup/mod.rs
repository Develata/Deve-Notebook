//! plan_ref:
//!   - 18_backup#backup-locator-contract
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-pack-contract
//!   - 18_backup#backup-restore-candidate-contract
//!
//! Backup runtime boundary.
//!
//! This module currently owns locator parsing, branch path derivation, and
//! branch backup binding validation, backup pack manifest planning/validation,
//! and restore candidate admission only. It does not open network connections,
//! write ledger state, modify staging, or touch Projection Workspaces.

mod binding;
mod locator;
mod pack;
mod restore;

pub use binding::{
    BackupBindingAccess, BackupBindingError, BackupBranchBinding, BackupBranchBindingInput,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};
pub use locator::{BackupLocator, BackupLocatorError, BackupProviderKind, BranchBackupLocator};
pub use pack::{
    BACKUP_PACK_FORMAT_VERSION, BackupBlobRef, BackupDigest, BackupPackError, BackupPackManifest,
    BackupPackPlanInput, BackupSeqRange, plan_backup_pack, validate_pack_manifest,
};
pub use restore::{
    BackupRestoreError, RestoreAdmissionMode, RestoreAdmissionState, RestoreCandidate,
    RestoreCandidateInput, RestoreEvidence, admit_restore_candidate,
};
