//! plan_ref:
//!   - 18_backup#backup-locator-contract
//!   - 18_backup#backup-pack-contract
//!   - 18_backup#backup-restore-candidate-contract
//!
//! Backup runtime boundary.
//!
//! This module currently owns locator parsing, branch path derivation, and
//! backup pack manifest planning/validation, and restore candidate admission
//! only. It does not open network connections, write ledger state, modify
//! staging, or touch Projection Workspaces.

mod locator;
mod pack;
mod restore;

pub use locator::{BackupLocator, BackupLocatorError, BackupProviderKind, BranchBackupLocator};
pub use pack::{
    BACKUP_PACK_FORMAT_VERSION, BackupBlobRef, BackupDigest, BackupPackError, BackupPackManifest,
    BackupPackPlanInput, BackupSeqRange, plan_backup_pack, validate_pack_manifest,
};
pub use restore::{
    BackupRestoreError, RestoreAdmissionMode, RestoreAdmissionState, RestoreCandidate,
    RestoreCandidateInput, RestoreEvidence, admit_restore_candidate,
};
