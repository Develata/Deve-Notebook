//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!
//! Downloaded pack evidence gate.
//!
//! This module checks that encrypted pack artifacts downloaded from provider
//! objects correspond exactly to a verified branch manifest. It does not fetch
//! providers, authenticate/decrypt artifacts, append ledger entries, stage
//! source-control changes, or touch Projection Workspaces.

use super::{BackupVerificationError, validate_digest};
use crate::backup::BackupPackArtifactDownloadVerifyResult;
use crate::backup::branch_manifest::{BackupBranchManifest, BackupBranchManifestPackRef};
use crate::backup::pack::BackupDigest;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupDownloadedPacksInput<'a> {
    pub branch_manifest: &'a BackupBranchManifest,
    pub verified_packs: Vec<BackupPackArtifactDownloadVerifyResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupDownloadedPacksResult {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
}

pub fn verify_downloaded_backup_packs(
    input: BackupDownloadedPacksInput<'_>,
) -> Result<BackupDownloadedPacksResult, BackupVerificationError> {
    if input.branch_manifest.packs.is_empty() || input.verified_packs.is_empty() {
        return Err(BackupVerificationError::EmptyPackList);
    }

    let expected_by_sequence = expected_pack_refs(input.branch_manifest)?;
    let mut seen_sequences = HashSet::with_capacity(input.verified_packs.len());
    let mut seen_paths = HashSet::with_capacity(input.verified_packs.len());

    for pack in input.verified_packs {
        if pack.pack_sequence() == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(pack.computed_digest())?;
        if !seen_sequences.insert(pack.pack_sequence()) {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
        if !seen_paths.insert(pack.object_path().to_owned()) {
            return Err(BackupVerificationError::DuplicatePackObjectPath);
        }

        let expected = expected_by_sequence
            .get(&pack.pack_sequence())
            .ok_or(BackupVerificationError::UnexpectedDownloadedPack)?;
        if pack.object_path() != expected.object_path {
            return Err(BackupVerificationError::PackObjectPathMismatch);
        }
        if !expected.payload_digest.same_sha256(pack.computed_digest()) {
            return Err(BackupVerificationError::PackHashMismatch);
        }
    }

    if seen_sequences.len() != expected_by_sequence.len() {
        return Err(BackupVerificationError::MissingDownloadedPack);
    }

    Ok(BackupDownloadedPacksResult {
        repo_id: input.branch_manifest.repo_id,
        writer_identity: input.branch_manifest.writer_identity.clone(),
        branch_path: input.branch_manifest.branch_path.clone(),
        pack_count: u64::try_from(input.branch_manifest.packs.len()).unwrap_or(u64::MAX),
        pack_digests: input
            .branch_manifest
            .packs
            .iter()
            .map(|pack| pack.payload_digest.clone())
            .collect(),
    })
}

fn expected_pack_refs(
    branch_manifest: &BackupBranchManifest,
) -> Result<HashMap<u64, &BackupBranchManifestPackRef>, BackupVerificationError> {
    let mut expected = HashMap::with_capacity(branch_manifest.packs.len());
    let mut object_paths = HashSet::with_capacity(branch_manifest.packs.len());
    for pack in &branch_manifest.packs {
        if pack.pack_sequence == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(&pack.payload_digest)?;
        if !object_paths.insert(pack.object_path.as_str()) {
            return Err(BackupVerificationError::DuplicatePackObjectPath);
        }
        if expected.insert(pack.pack_sequence, pack).is_some() {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
    }
    Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::{
        BackupArtifactKey, BackupArtifactKind, BackupArtifactProtection,
        BackupArtifactProtectionInput, BackupBlobRef, BackupEncryptedPackArtifact,
        BackupPackArtifactDownloadVerifyInput, BackupPackArtifactInput, BackupPackManifest,
        BackupPackPlanInput, BackupProtectionMechanism, BackupSeqRange,
        encrypt_backup_pack_artifact, parse_backup_key_ref, plan_backup_artifact_protection,
        plan_backup_pack, verify_downloaded_pack_artifact_digest_and_routing,
    };

    fn digest(fill: char) -> BackupDigest {
        BackupDigest::sha256(fill.to_string().repeat(64))
    }

    fn repo_id() -> RepoId {
        uuid::Uuid::from_u128(23)
    }

    fn artifact_key() -> BackupArtifactKey {
        BackupArtifactKey::from_bytes(&[7; 32]).unwrap()
    }

    fn protection() -> BackupArtifactProtection {
        plan_backup_artifact_protection(BackupArtifactProtectionInput {
            artifact_kind: BackupArtifactKind::Pack,
            key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
            encrypted: true,
            authenticated: true,
            mechanism: BackupProtectionMechanism::AeadTag,
        })
        .unwrap()
    }

    fn pack_artifact_input<'a>(
        key: &'a BackupArtifactKey,
        protection: &'a BackupArtifactProtection,
        pack_sequence: u64,
        writer_identity: &'a str,
        branch_path: &'a str,
        plaintext: &'a [u8],
    ) -> BackupPackArtifactInput<'a> {
        BackupPackArtifactInput {
            repo_id: repo_id(),
            writer_identity,
            branch_path,
            pack_sequence,
            protection,
            key,
            plaintext,
        }
    }

    fn pack_manifest_for(artifact: &BackupEncryptedPackArtifact) -> BackupPackManifest {
        let payload_digest = artifact.payload_digest().unwrap();
        plan_backup_pack(BackupPackPlanInput {
            repo_id: artifact.repo_id,
            writer_identity: artifact.writer_identity.clone(),
            branch_path: artifact.branch_path.clone(),
            pack_sequence: artifact.pack_sequence,
            ledger_seq_range: Some(BackupSeqRange {
                start: artifact.pack_sequence,
                end: artifact.pack_sequence,
            }),
            ledger_event_count: 1,
            snapshot_count: 1,
            payload_digest: payload_digest.clone(),
            blob_refs: vec![BackupBlobRef {
                path: format!("blobs/{:06}.bin", artifact.pack_sequence),
                size_bytes: 12,
                digest: payload_digest,
            }],
        })
        .unwrap()
    }

    fn verified_pack(
        pack_sequence: u64,
        writer_identity: &str,
        plaintext: &[u8],
    ) -> BackupPackArtifactDownloadVerifyResult {
        let key = artifact_key();
        let protection = protection();
        let branch_path = format!("deve/branches/{writer_identity}");
        let artifact = encrypt_backup_pack_artifact(pack_artifact_input(
            &key,
            &protection,
            pack_sequence,
            writer_identity,
            &branch_path,
            plaintext,
        ))
        .unwrap();
        let artifact_bytes = artifact.to_bytes().unwrap();
        let manifest = pack_manifest_for(&artifact);

        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap()
    }

    fn verified_packs() -> Vec<BackupPackArtifactDownloadVerifyResult> {
        vec![
            verified_pack(1, "writer-1", b"ledger facts one"),
            verified_pack(2, "writer-1", b"ledger facts two"),
        ]
    }

    fn branch_manifest(
        verified_packs: &[BackupPackArtifactDownloadVerifyResult],
    ) -> BackupBranchManifest {
        BackupBranchManifest {
            repo_id: repo_id(),
            writer_identity: "writer-1".into(),
            branch_path: "deve/branches/writer-1".into(),
            branch_manifest_path: "deve/branches/writer-1/branch.manifest.enc".into(),
            pack_prefix: "deve/branches/writer-1/packs".into(),
            format_version: crate::backup::BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
            packs: verified_packs
                .iter()
                .map(|pack| BackupBranchManifestPackRef {
                    pack_sequence: pack.pack_sequence(),
                    object_path: pack.object_path().to_owned(),
                    payload_digest: pack.computed_digest().clone(),
                })
                .collect(),
        }
    }

    #[test]
    fn backup_downloaded_packs_match_manifest_refs() {
        let verified_packs = verified_packs();
        let manifest = branch_manifest(&verified_packs);

        let result = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: verified_packs.clone(),
        })
        .expect("downloaded pack evidence");

        assert_eq!(result.repo_id, manifest.repo_id);
        assert_eq!(result.writer_identity, "writer-1");
        assert_eq!(result.branch_path, "deve/branches/writer-1");
        assert_eq!(result.pack_count, 2);
        assert_eq!(
            result.pack_digests,
            verified_packs
                .iter()
                .map(|pack| pack.computed_digest().clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn backup_downloaded_packs_accept_provider_order_independent_verified_results() {
        let verified_packs = verified_packs();
        let manifest = branch_manifest(&verified_packs);
        let mut reversed = verified_packs.clone();
        reversed.reverse();

        let result = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: reversed,
        })
        .expect("provider order must not affect branch manifest evidence");

        assert_eq!(
            result.pack_digests,
            verified_packs
                .iter()
                .map(|pack| pack.computed_digest().clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn backup_downloaded_packs_consume_verified_artifact_results_only() {
        let verified_pack = verified_pack(1, "writer-1", b"verified pack");
        let manifest = branch_manifest(std::slice::from_ref(&verified_pack));

        let result = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: vec![verified_pack.clone()],
        })
        .expect("artifact download verification result is the gate input");

        assert_eq!(result.pack_count, 1);
        assert_eq!(
            result.pack_digests,
            vec![verified_pack.computed_digest().clone()]
        );
    }

    #[test]
    fn backup_downloaded_packs_reject_missing_or_unexpected_pack() {
        let verified_packs = verified_packs();
        let manifest = branch_manifest(&verified_packs);
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: vec![verified_packs[0].clone()],
        })
        .expect_err("missing downloaded pack must fail closed");
        assert_eq!(err, BackupVerificationError::MissingDownloadedPack);

        let unexpected_pack = verified_pack(99, "writer-1", b"unexpected pack");
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: vec![verified_packs[0].clone(), unexpected_pack],
        })
        .expect_err("unexpected downloaded pack must fail closed");
        assert_eq!(err, BackupVerificationError::UnexpectedDownloadedPack);
    }

    #[test]
    fn backup_downloaded_packs_reject_path_or_digest_mismatch() {
        let verified_packs = verified_packs();
        let manifest = branch_manifest(&verified_packs);
        let wrong_path = verified_pack(1, "writer-2", b"ledger facts one");
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: vec![wrong_path, verified_packs[1].clone()],
        })
        .expect_err("wrong object path must fail closed");
        assert_eq!(err, BackupVerificationError::PackObjectPathMismatch);

        let mut manifest = branch_manifest(&verified_packs);
        manifest.packs[0].payload_digest = digest('f');
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs,
        })
        .expect_err("wrong digest must fail closed");
        assert_eq!(err, BackupVerificationError::PackHashMismatch);
    }

    #[test]
    fn backup_downloaded_packs_reject_duplicate_sequence_or_path() {
        let verified_packs = verified_packs();
        let manifest = branch_manifest(&verified_packs);
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs: vec![verified_packs[0].clone(), verified_packs[0].clone()],
        })
        .expect_err("duplicate sequence must fail closed");
        assert_eq!(err, BackupVerificationError::DuplicatePackSequence);

        let mut manifest = branch_manifest(&verified_packs);
        manifest.packs[1].object_path = manifest.packs[0].object_path.clone();
        let err = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
            branch_manifest: &manifest,
            verified_packs,
        })
        .expect_err("duplicate path must fail closed");
        assert_eq!(err, BackupVerificationError::DuplicatePackObjectPath);
    }
}
