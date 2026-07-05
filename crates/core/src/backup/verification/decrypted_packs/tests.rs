use super::*;
use crate::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupArtifactKey, BackupArtifactKind,
    BackupArtifactProtection, BackupArtifactProtectionInput, BackupBlobRef, BackupBranchManifest,
    BackupBranchManifestPackRef, BackupEncryptedPackArtifact,
    BackupPackArtifactDownloadVerifyInput, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackManifest, BackupPackPlanInput,
    BackupProtectionMechanism, BackupSeqRange, BackupVerificationError,
    encrypt_backup_pack_artifact, open_backup_pack_artifact, parse_backup_key_ref,
    plan_backup_artifact_protection, plan_backup_pack, verify_downloaded_backup_packs,
    verify_downloaded_pack_artifact_digest_and_routing,
};

struct PackFixture {
    download_result: crate::backup::BackupPackArtifactDownloadVerifyResult,
    open_result: BackupPackArtifactOpenResult,
}

fn repo_id() -> RepoId {
    uuid::Uuid::from_u128(29)
}

fn artifact_key() -> BackupArtifactKey {
    BackupArtifactKey::from_bytes(&[9; 32]).unwrap()
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

fn pack_fixture(pack_sequence: u64, writer_identity: &str, plaintext: &[u8]) -> PackFixture {
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
    let download_result =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();
    let open_result = open_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    PackFixture {
        download_result,
        open_result,
    }
}

fn pack_fixtures() -> Vec<PackFixture> {
    vec![
        pack_fixture(1, "writer-1", b"ledger facts one"),
        pack_fixture(2, "writer-1", b"ledger facts two"),
    ]
}

fn branch_manifest(fixtures: &[PackFixture]) -> BackupBranchManifest {
    BackupBranchManifest {
        repo_id: repo_id(),
        writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1".into(),
        branch_manifest_path: "deve/branches/writer-1/branch.manifest.enc".into(),
        pack_prefix: "deve/branches/writer-1/packs".into(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: fixtures
            .iter()
            .map(|fixture| BackupBranchManifestPackRef {
                pack_sequence: fixture.download_result.pack_sequence(),
                object_path: fixture.download_result.object_path().to_owned(),
                payload_digest: fixture.download_result.computed_digest().clone(),
            })
            .collect(),
    }
}

fn downloaded_result(fixtures: &[PackFixture]) -> BackupDownloadedPacksResult {
    let manifest = branch_manifest(fixtures);
    verify_downloaded_backup_packs(crate::backup::BackupDownloadedPacksInput {
        branch_manifest: &manifest,
        verified_packs: fixtures
            .iter()
            .map(|fixture| fixture.download_result.clone())
            .collect(),
    })
    .unwrap()
}

#[test]
fn backup_decrypted_packs_match_downloaded_pack_refs() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let expected_encrypted_bytes = fixtures
        .iter()
        .map(|fixture| fixture.open_result.encrypted_bytes())
        .sum::<usize>();
    let expected_plaintext_bytes = fixtures
        .iter()
        .map(|fixture| fixture.open_result.plaintext().len())
        .sum::<usize>();

    let result = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .expect("decrypted pack evidence");

    assert_eq!(result.repo_id(), repo_id());
    assert_eq!(result.writer_identity(), "writer-1");
    assert_eq!(result.branch_path(), "deve/branches/writer-1");
    assert_eq!(result.pack_count(), 2);
    assert_eq!(result.pack_digests(), downloaded.pack_digests());
    assert_eq!(result.encrypted_bytes_total(), expected_encrypted_bytes);
    assert_eq!(result.plaintext_bytes_total(), expected_plaintext_bytes);
    assert!(result.plaintext_packs()[0].encrypted_bytes() > 0);
    assert_eq!(result.plaintext_packs()[0].plaintext(), b"ledger facts one");
    assert_eq!(result.plaintext_packs()[1].plaintext(), b"ledger facts two");
}

#[test]
fn backup_decrypted_packs_accept_provider_order_independent_open_results() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let mut opened_packs = fixtures
        .into_iter()
        .map(|fixture| fixture.open_result)
        .collect::<Vec<_>>();
    opened_packs.reverse();

    let result = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs,
    })
    .expect("open result order must not affect decrypted evidence");

    assert_eq!(result.plaintext_packs()[0].pack_sequence(), 1);
    assert_eq!(result.plaintext_packs()[1].pack_sequence(), 2);
}

#[test]
fn backup_decrypted_packs_consume_open_artifact_results_only() {
    let fixtures = vec![pack_fixture(1, "writer-1", b"verified decrypted pack")];
    let downloaded = downloaded_result(&fixtures);

    let result = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .expect("artifact open result is the decrypted gate input");

    assert_eq!(result.pack_count(), 1);
    assert_eq!(
        result.plaintext_packs()[0].plaintext(),
        b"verified decrypted pack"
    );
}

#[test]
fn backup_decrypted_packs_reject_missing_or_unexpected_pack() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let err = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: vec![fixtures.into_iter().next().unwrap().open_result],
    })
    .expect_err("missing decrypted pack must fail closed");
    assert_eq!(err, BackupVerificationError::MissingDecryptedPack);

    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let unexpected = pack_fixture(99, "writer-1", b"unexpected decrypted pack");
    let err = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: vec![
            fixtures.into_iter().next().unwrap().open_result,
            unexpected.open_result,
        ],
    })
    .expect_err("unexpected decrypted pack must fail closed");
    assert_eq!(err, BackupVerificationError::UnexpectedDecryptedPack);
}

#[test]
fn backup_decrypted_packs_reject_path_or_digest_mismatch() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let wrong_path = pack_fixture(1, "writer-2", b"ledger facts one");
    let err = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: vec![
            wrong_path.open_result,
            fixtures.into_iter().nth(1).unwrap().open_result,
        ],
    })
    .expect_err("wrong object path must fail closed");
    assert_eq!(err, BackupVerificationError::PackObjectPathMismatch);

    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let digest_mismatch = pack_fixture(1, "writer-1", b"different plaintext");
    let err = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: vec![
            digest_mismatch.open_result,
            fixtures.into_iter().nth(1).unwrap().open_result,
        ],
    })
    .expect_err("wrong encrypted digest must fail closed");
    assert_eq!(err, BackupVerificationError::PackHashMismatch);
}

#[test]
fn backup_decrypted_packs_reject_duplicate_sequence() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let duplicate = pack_fixture(1, "writer-1", b"duplicate sequence");
    let err = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: vec![
            fixtures.into_iter().next().unwrap().open_result,
            duplicate.open_result,
        ],
    })
    .expect_err("duplicate decrypted sequence must fail closed");
    assert_eq!(err, BackupVerificationError::DuplicatePackSequence);
}

#[test]
fn backup_decrypted_packs_open_result_cannot_be_empty_plaintext() {
    let fixtures = pack_fixtures();
    let downloaded = downloaded_result(&fixtures);
    let key = artifact_key();
    let protection = protection();
    assert!(matches!(
        encrypt_backup_pack_artifact(pack_artifact_input(
            &key,
            &protection,
            3,
            "writer-1",
            "deve/branches/writer-1",
            b""
        )),
        Err(crate::backup::BackupPackArtifactError::EmptyPlaintext)
    ));

    let result = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .expect("non-empty decrypted fixtures remain accepted");
    assert!(
        result
            .plaintext_packs()
            .iter()
            .all(|pack| !pack.plaintext().is_empty())
    );
}
