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
mod plaintext;
mod protection;
mod provider;
mod restore;
mod restore_flow;
mod root;
mod secret;
mod upload;
mod verification;

pub use artifact::{
    BackupArtifactKey, BackupEncryptedPackArtifact, BackupPackArtifactDownloadVerifyInput,
    BackupPackArtifactDownloadVerifyResult, BackupPackArtifactError, BackupPackArtifactInput,
    BackupPackArtifactOpenInput, BackupPackArtifactOpenResult,
    BackupPackArtifactRefDownloadVerifyInput, BackupPackArtifactRefOpenInput,
    BackupPackArtifactUploadVerifyInput, decrypt_backup_pack_artifact,
    encrypt_backup_pack_artifact, open_backup_pack_artifact, open_backup_pack_artifact_ref,
    verify_backup_pack_artifact_for_upload, verify_downloaded_pack_artifact_digest_and_routing,
    verify_downloaded_pack_artifact_ref_and_routing,
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
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBranchManifest, BackupBranchManifestArtifactError,
    BackupBranchManifestArtifactInput, BackupBranchManifestArtifactOpenInput,
    BackupBranchManifestArtifactOpenResult, BackupBranchManifestError, BackupBranchManifestInput,
    BackupBranchManifestPackRef, BackupEncryptedBranchManifestArtifact,
    encrypt_backup_branch_manifest_artifact, open_backup_branch_manifest_artifact,
    validate_backup_branch_manifest,
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
pub use plaintext::{
    BACKUP_PACK_PLAINTEXT_FORMAT_VERSION, BackupPackPlaintext, BackupPackPlaintextEncodeInput,
    BackupPackPlaintextError, BackupPackPlaintextLedgerEntry, BackupPackPlaintextOpenInput,
    BackupPackPlaintextValidateInput, encode_backup_pack_plaintext, open_backup_pack_plaintext,
    validate_backup_pack_plaintext,
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
    BACKUP_RESTORE_MAX_ENCRYPTED_BYTES, BACKUP_RESTORE_MAX_PACKS,
    BACKUP_RESTORE_MAX_PLAINTEXT_BYTES, BackupRestoreError, BackupRestoreResourceBudgetInput,
    RestoreAdmissionMode, RestoreAdmissionState, RestoreCandidate,
    RestoreCandidateFromVerifiedPacksInput, admit_verified_restore_candidate,
    validate_backup_restore_resource_budget,
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
    BackupDecryptedPackPayload, BackupDecryptedPacksInput, BackupDecryptedPacksResult,
    BackupDownloadedPackRef, BackupDownloadedPacksInput, BackupDownloadedPacksResult,
    BackupPackVerificationEvidence, BackupPlaintextPackPayload, BackupPlaintextPacksInput,
    BackupPlaintextPacksResult, BackupVerificationError, BackupVerificationInput,
    BackupVerificationResult, BackupVerifiedPackRef, verify_backup_artifacts,
    verify_decrypted_backup_packs, verify_downloaded_backup_packs, verify_plaintext_backup_packs,
};
