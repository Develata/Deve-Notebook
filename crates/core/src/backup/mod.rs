//! plan_ref:
//!   - 06_backup#backup-locator-contract
//!   - 06_backup#backup-remote-layout-contract
//!   - 06_backup#backup-root-contract
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-restore-candidate-contract
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-secret-ref-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup runtime boundary.
//!
//! This module currently owns locator parsing, branch path derivation, and
//! branch backup binding validation, backup pack manifest planning/validation,
//! encrypted pack artifact sealing/opening, branch manifest validation,
//! readonly branch discovery, artifact protection admission, provider adapter
//! dispatch, upload state admission, verification evidence validation, command
//! output modeling, restore flow admission, and restore candidate admission
//! only. It does not open network connections, write ledger state, modify
//! staging, or touch Projection Workspaces.

mod artifact;
mod binding;
mod binding_store;
mod branch_manifest;
mod discovery;
mod layout;
mod locator;
mod output;
mod pack;
mod protection;
mod provider;
mod restore;
mod restore_flow;
mod root;
mod secret;
mod upload;
mod verification;

pub use artifact::{
    BackupArtifactKey, BackupEncryptedPackArtifact, BackupPackArtifactError,
    BackupPackArtifactInput, BackupPackArtifactOpenInput, decrypt_backup_pack_artifact,
    encrypt_backup_pack_artifact,
};
pub use binding::{
    BackupBindingAccess, BackupBindingError, BackupBranchBinding, BackupBranchBindingInput,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};
pub use binding_store::{
    BackupBindingRecord, BackupBindingStoreError, backup_binding_store_path_for,
    list_backup_binding_records, persist_backup_branch_binding, remove_backup_branch_binding,
};
pub use branch_manifest::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBranchManifest, BackupBranchManifestError,
    BackupBranchManifestInput, BackupBranchManifestPackRef, validate_backup_branch_manifest,
};
pub use discovery::{
    BackupBranchDiscoveryDiagnostic, BackupBranchDiscoveryDiagnosticKind,
    BackupBranchDiscoveryInput, BackupBranchDiscoveryReport, DiscoveredBackupBranch,
    discover_backup_branches,
};
pub use layout::{
    BackupRemoteLayoutDiagnostic, BackupRemoteLayoutDiagnosticKind, BackupRemoteLayoutError,
    BackupRemoteLayoutInput, BackupRemoteLayoutReport, BackupRemoteObject, BackupTransportMetadata,
    inspect_backup_remote_layout,
};
pub use locator::{BackupLocator, BackupLocatorError, BackupProviderKind, BranchBackupLocator};
pub use output::{
    BackupBindingStatus, BackupCommandKind, BackupCommandOutputError, BackupError, BackupErrorKind,
    BackupPlan, BackupPlanEffect, BackupPlanInput, backup_binding_status, backup_command_plan,
};
pub use pack::{
    BACKUP_PACK_FORMAT_VERSION, BackupBlobRef, BackupDigest, BackupPackError, BackupPackManifest,
    BackupPackPlanInput, BackupSeqRange, plan_backup_pack, validate_pack_manifest,
};
pub use protection::{
    BackupArtifactKind, BackupArtifactProtection, BackupArtifactProtectionError,
    BackupArtifactProtectionInput, BackupProtectionMechanism, plan_backup_artifact_protection,
};
pub use provider::{
    BackupProviderAdapterPlan, BackupProviderDispatchError, BackupProviderDispatchInput,
    dispatch_backup_provider_adapter,
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
