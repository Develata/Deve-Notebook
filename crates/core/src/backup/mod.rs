//! plan_ref:
//!   - 18_backup#backup-locator-contract
//!   - 18_backup#backup-remote-layout-contract
//!   - 18_backup#backup-root-contract
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-pack-contract
//!   - 18_backup#backup-upload-state-machine-contract
//!   - 18_backup#backup-restore-candidate-contract
//!   - 18_backup#backup-restore-state-machine-contract
//!   - 18_backup#backup-secret-ref-contract
//!   - 18_backup#backup-verification-contract
//!
//! Backup runtime boundary.
//!
//! This module currently owns locator parsing, branch path derivation, and
//! branch backup binding validation, backup pack manifest planning/validation,
//! upload state admission, verification evidence validation, restore flow
//! admission, and restore candidate admission only. It does not open network
//! connections, write ledger state, modify staging, or touch Projection
//! Workspaces.

mod binding;
mod layout;
mod locator;
mod pack;
mod restore;
mod restore_flow;
mod root;
mod secret;
mod upload;
mod verification;

pub use binding::{
    BackupBindingAccess, BackupBindingError, BackupBranchBinding, BackupBranchBindingInput,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};
pub use layout::{
    BackupRemoteLayoutDiagnostic, BackupRemoteLayoutDiagnosticKind, BackupRemoteLayoutError,
    BackupRemoteLayoutInput, BackupRemoteLayoutReport, BackupRemoteObject, BackupTransportMetadata,
    inspect_backup_remote_layout,
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
pub use restore_flow::{
    BackupRestoreFlowError, BackupRestoreFlowEvidence, BackupRestoreFlowInput,
    BackupRestoreFlowPlan, BackupRestoreFlowState, plan_backup_restore_flow,
};
pub use root::{
    BACKUP_ROOT_FORMAT_VERSION, BackupRoot, BackupRootError, BackupRootInput, plan_backup_root,
};
pub use secret::{
    BackupSecretRef, BackupSecretRefError, BackupSecretRefKind, BackupSecretRefScheme,
    parse_backup_credential_ref, parse_backup_key_ref,
};
pub use upload::{
    BackupUploadError, BackupUploadEvidence, BackupUploadPlan, BackupUploadPlanInput,
    BackupUploadState, plan_backup_upload,
};
pub use verification::{
    BackupPackVerificationEvidence, BackupVerificationError, BackupVerificationInput,
    BackupVerificationResult, verify_backup_artifacts,
};
