use super::super::super::restore_lines_with_runtime;
use super::super::support::{
    FixedKeyResolver, REPO_ID, RecordingDownloader, artifact_key, download_fixture,
    download_fixture_with_pack_count, download_fixture_with_pack_key, download_input,
    encrypted_pack_fixture_with_plaintext, protection,
};
use deve_core::backup::{
    BACKUP_RESTORE_MAX_PACKS, BackupArtifactKey, BackupArtifactKind,
    BackupBranchManifestArtifactInput, BackupLocator, encrypt_backup_branch_manifest_artifact,
};

#[test]
fn backup_restore_download_verifies_pack_plaintext_schema_before_candidate() {
    let key = artifact_key();
    let (pack_ref, pack_bytes) =
        encrypted_pack_fixture_with_plaintext(&key, 1, b"raw plaintext without schema");
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();
    let branch_manifest_path = branch.branch_manifest_path();
    let branch_manifest =
        encrypt_backup_branch_manifest_artifact(BackupBranchManifestArtifactInput {
            branch,
            repo_id: REPO_ID.parse().expect("repo id"),
            writer_identity: "writer-1",
            branch_path: "deve/branches/writer-1",
            packs: vec![pack_ref.clone()],
            protection: &protection(BackupArtifactKind::BranchManifest),
            key: &key,
        })
        .expect("encrypted branch manifest");
    let manifest_digest = branch_manifest
        .payload_digest()
        .expect("manifest digest")
        .hex;
    let artifacts = vec![
        (
            branch_manifest_path,
            branch_manifest.to_bytes().expect("manifest bytes"),
        ),
        (pack_ref.object_path.clone(), pack_bytes),
    ];
    let pack_digests = vec![pack_ref.payload_digest.hex.clone()];
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("raw plaintext must fail before candidate admission");

    assert!(err.to_string().contains("plaintext"));
    assert_eq!(downloader.requests.len(), 2);
}

#[test]
fn backup_restore_download_rejects_wrong_key_before_candidate() {
    let wrong_pack_key = BackupArtifactKey::from_bytes(&[8; 32]).expect("wrong pack key");
    let (manifest_key, manifest_digest, artifacts, pack_digests) =
        download_fixture_with_pack_key(2, &wrong_pack_key, artifact_key());
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(manifest_key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("wrong key must fail before candidate admission");

    assert!(err.to_string().contains("decryption failed"));
    assert_eq!(key_resolver.requests, vec!["env:<redacted>".to_string()]);
    assert_eq!(downloader.requests.len(), 3);
}

#[test]
fn backup_restore_download_rejects_resource_budget_excess() {
    let pack_count = BACKUP_RESTORE_MAX_PACKS + 1;
    let (key, manifest_digest, artifacts, pack_digests) =
        download_fixture_with_pack_count(pack_count);
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("resource budget excess must fail closed before pack download");

    assert!(err.to_string().contains("resource budget"));
    assert_eq!(downloader.requests.len(), 1);
}

#[test]
fn backup_restore_download_rejects_tampered_artifact_before_candidate() {
    let (key, manifest_digest, mut artifacts, pack_digests) = download_fixture();
    for (path, bytes) in &mut artifacts {
        if path.ends_with("000001.pack.enc") {
            bytes.push(b'\n');
        }
    }
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("tampered artifact must fail before candidate admission");

    assert!(err.to_string().contains("digest"));
    assert_eq!(downloader.requests.len(), 2);
}

#[test]
fn backup_restore_download_rejects_authoritative_provider_metadata() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts).with_authoritative_metadata();
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("provider metadata must remain diagnostic only");

    assert!(err.to_string().contains("diagnostic-only"));
    assert_eq!(downloader.requests.len(), 1);
}
