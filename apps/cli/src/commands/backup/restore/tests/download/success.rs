use super::super::super::restore_lines_with_runtime;
use super::super::support::{
    DownloadRecord, FixedKeyResolver, RecordingDownloader, download_fixture, download_input,
    restore_with_fixture,
};
use crate::commands::backup::provider_io::BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES;

#[test]
fn backup_restore_download_opens_branch_manifest_before_pack_download() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, downloader, key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert_eq!(
        downloader.requests,
        vec![
            DownloadRecord {
                object_path: "deve/branches/writer-1/branch.manifest.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
            DownloadRecord {
                object_path: "deve/branches/writer-1/packs/000001.pack.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
            DownloadRecord {
                object_path: "deve/branches/writer-1/packs/000002.pack.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
        ]
    );
    assert_eq!(key_resolver.requests, vec!["env:<redacted>".to_string()]);
    assert!(lines.iter().any(|line| line == "artifact_io=true"));
    assert!(lines.iter().any(|line| line == "manifest_verified=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=RestoreCandidate")
    );
    assert!(lines.iter().any(|line| line == "packs_decrypted=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == "pack_plaintext_schema_verified=true")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "candidate_admission=created_remote_readonly")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_candidate_state=RemoteReadonly")
    );
}

#[test]
fn backup_restore_download_selects_pack_from_branch_manifest() {
    let (key, manifest_digest, artifacts, _pack_digests) = download_fixture();
    let empty_pack_digests = Vec::new();
    let command = download_input(&manifest_digest, &empty_pack_digests);
    let (lines, _downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert!(lines.iter().any(|line| line == "pack_count=2"));
    assert!(lines.iter().any(|line| line == "verified_pack_sequence=1"));
    assert!(lines.iter().any(|line| line == "verified_pack_sequence=2"));
    assert!(lines.iter().any(|line| {
        line == "verified_pack_object_path=deve/branches/writer-1/packs/000001.pack.enc"
    }));
    assert!(lines.iter().any(|line| {
        line == "verified_pack_object_path=deve/branches/writer-1/packs/000002.pack.enc"
    }));
}

#[test]
fn backup_restore_download_opens_pack_artifacts_from_branch_manifest_refs() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert_eq!(downloader.requests.len(), 3);
    assert!(lines.iter().any(|line| line == "packs_decrypted=true"));
    assert!(lines.iter().any(|line| line == "pack_count=2"));
    assert!(
        lines
            .iter()
            .any(|line| line == "candidate_admission=created_remote_readonly")
    );
}

#[test]
fn backup_restore_download_admits_remote_readonly_candidate_after_pack_decrypt() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, _downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=RestoreCandidate")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_candidate_state=RemoteReadonly")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

#[test]
fn backup_restore_download_budget_uses_actual_artifact_bytes_not_provider_count() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let expected_downloaded_bytes: usize = artifacts.iter().map(|(_, bytes)| bytes.len()).sum();
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader =
        RecordingDownloader::new(artifacts).with_reported_downloaded_bytes(usize::MAX);
    let mut key_resolver = FixedKeyResolver::new(key);
    let lines = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect("provider reported byte count must remain diagnostic-only");

    let expected_downloaded_bytes_line = format!("downloaded_bytes={expected_downloaded_bytes}");
    assert!(
        lines
            .iter()
            .any(|line| line == &expected_downloaded_bytes_line)
    );
    assert!(
        !lines
            .iter()
            .any(|line| line == &format!("downloaded_bytes={}", usize::MAX))
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=true")
    );
}
