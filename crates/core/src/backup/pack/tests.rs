use super::{
    BACKUP_PACK_FORMAT_VERSION, BackupBlobRef, BackupDigest, BackupPackError, BackupPackPlanInput,
    BackupSeqRange, plan_backup_pack, validate_pack_manifest,
};
use crate::backup::BackupLocator;

fn digest() -> BackupDigest {
    BackupDigest::sha256("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
}

fn uppercase_digest() -> BackupDigest {
    BackupDigest::sha256("0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF")
}

fn plan_input() -> BackupPackPlanInput {
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    BackupPackPlanInput {
        repo_id: uuid::Uuid::from_u128(7),
        writer_identity: branch.writer_identity,
        branch_path: branch.branch_path,
        pack_sequence: 1,
        ledger_seq_range: Some(BackupSeqRange { start: 3, end: 5 }),
        ledger_event_count: 3,
        snapshot_count: 1,
        payload_digest: digest(),
        blob_refs: vec![BackupBlobRef {
            path: "blobs/aa.bin".into(),
            size_bytes: 12,
            digest: digest(),
        }],
    }
}

#[test]
fn plans_manifest_with_remote_layout_paths() {
    let manifest = plan_backup_pack(plan_input()).unwrap();

    assert_eq!(manifest.format_version, BACKUP_PACK_FORMAT_VERSION);
    assert_eq!(manifest.pack_file_name, "000001.pack.enc");
    assert_eq!(
        manifest.pack_object_path(),
        "deve/branches/writer-1/packs/000001.pack.enc"
    );
    assert_eq!(manifest.blob_refs[0].path, "blobs/aa.bin");
}

#[test]
fn compares_sha256_digests_by_canonical_hex() {
    assert!(digest().same_sha256(&uppercase_digest()));
    assert_eq!(
        uppercase_digest().canonical_sha256_hex().as_deref(),
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
}

#[test]
fn validates_manifest_against_repo_writer_and_branch() {
    let input = plan_input();
    let manifest = plan_backup_pack(input.clone()).unwrap();

    validate_pack_manifest(
        &manifest,
        input.repo_id,
        &input.writer_identity,
        &input.branch_path,
    )
    .unwrap();
}

#[test]
fn rejects_repo_or_branch_mismatch() {
    let input = plan_input();
    let manifest = plan_backup_pack(input.clone()).unwrap();

    assert!(matches!(
        validate_pack_manifest(
            &manifest,
            uuid::Uuid::from_u128(8),
            &input.writer_identity,
            &input.branch_path,
        ),
        Err(BackupPackError::RepoIdMismatch)
    ));
    assert!(matches!(
        validate_pack_manifest(&manifest, input.repo_id, "writer-1", "other/branch"),
        Err(BackupPackError::BranchPathMismatch)
    ));
}

#[test]
fn rejects_empty_or_invalid_ledger_range() {
    let mut input = plan_input();
    input.ledger_event_count = 0;
    input.snapshot_count = 0;
    input.blob_refs.clear();
    input.ledger_seq_range = None;
    assert!(matches!(
        plan_backup_pack(input),
        Err(BackupPackError::EmptyPack)
    ));

    let mut input = plan_input();
    input.ledger_seq_range = Some(BackupSeqRange { start: 6, end: 5 });
    assert!(matches!(
        plan_backup_pack(input),
        Err(BackupPackError::InvalidLedgerRange)
    ));
}

#[test]
fn rejects_invalid_digest_and_unsafe_blob_path() {
    let mut input = plan_input();
    input.payload_digest = BackupDigest::sha256("not-hex");
    assert!(matches!(
        plan_backup_pack(input),
        Err(BackupPackError::InvalidDigest)
    ));

    let mut input = plan_input();
    input.blob_refs[0].path = "blobs\\aa.bin".into();
    assert!(plan_backup_pack(input).is_err());
}

#[test]
fn rejects_duplicate_blob_paths_in_plan_and_manifest_validation() {
    let mut input = plan_input();
    input.blob_refs.push(BackupBlobRef {
        path: "blobs/aa.bin".into(),
        size_bytes: 99,
        digest: digest(),
    });

    assert!(matches!(
        plan_backup_pack(input),
        Err(BackupPackError::DuplicateBlobPath)
    ));

    let input = plan_input();
    let mut manifest = plan_backup_pack(input.clone()).unwrap();
    manifest.blob_refs.push(manifest.blob_refs[0].clone());

    assert!(matches!(
        validate_pack_manifest(
            &manifest,
            input.repo_id,
            &input.writer_identity,
            &input.branch_path,
        ),
        Err(BackupPackError::DuplicateBlobPath)
    ));
}
