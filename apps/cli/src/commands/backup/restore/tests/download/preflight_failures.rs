use super::super::super::restore_lines_with_runtime;
use super::super::support::{
    DIGEST_A, FixedKeyResolver, ForbiddenFlagCase, RecordingDownloader, download_fixture,
    download_input, restore_with_fixture,
};

#[test]
fn backup_restore_download_verifies_branch_manifest_digest_and_routing() {
    let (key, _manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(DIGEST_A, &pack_digests);
    let err = restore_with_fixture(command, artifacts, key)
        .expect_err("manifest digest mismatch must fail closed");

    assert!(err.to_string().contains("branch manifest artifact digest"));
}

#[test]
fn backup_restore_download_rejects_manual_decrypt_evidence_before_provider_get() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.packs_decrypted = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("download branch must reject precomputed decrypted evidence");

    assert!(err.to_string().contains("precomputed evidence"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_download_rejects_manual_evidence_before_provider_get() {
    let forbidden_flags: [ForbiddenFlagCase; 3] = [
        ("manifest", |command| command.manifest_verified = true),
        ("downloaded", |command| command.packs_downloaded = true),
        ("decrypted", |command| command.packs_decrypted = true),
    ];

    for (label, apply_flag) in forbidden_flags {
        let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
        let mut command = download_input(&manifest_digest, &pack_digests);
        apply_flag(&mut command);
        let mut downloader = RecordingDownloader::new(artifacts);
        let mut key_resolver = FixedKeyResolver::new(key);
        let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
            .expect_err(&format!("manual {label} evidence must fail closed"));

        assert!(err.to_string().contains("precomputed evidence"));
        assert!(downloader.requests.is_empty());
    }
}

#[test]
fn backup_restore_download_rejects_manual_pack_metadata_before_provider_get() {
    let forbidden_flags: [ForbiddenFlagCase; 5] = [
        ("pack_sequence", |command| command.pack_sequence = Some(1)),
        ("ledger_start", |command| command.ledger_start = Some(1)),
        ("ledger_end", |command| command.ledger_end = Some(1)),
        ("ledger_events", |command| {
            command.ledger_event_count = Some(1)
        }),
        ("snapshot_count", |command| command.snapshot_count = Some(0)),
    ];

    for (label, apply_flag) in forbidden_flags {
        let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
        let mut command = download_input(&manifest_digest, &pack_digests);
        apply_flag(&mut command);
        let mut downloader = RecordingDownloader::new(artifacts);
        let mut key_resolver = FixedKeyResolver::new(key);
        let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
            .expect_err(&format!("manual {label} metadata must fail closed"));

        assert!(err.to_string().contains("branch.manifest.enc"));
        assert!(downloader.requests.is_empty());
    }
}

#[test]
fn backup_restore_download_rejects_metadata_before_provider_get() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.manifest_repo_id = "22222222-2222-2222-2222-222222222222";
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("manifest mismatch must fail before provider I/O");

    assert!(err.to_string().contains("repo id"));
    assert!(downloader.requests.is_empty());

    let (key, _manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input("not-a-sha256-digest", &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("invalid manifest digest must fail before provider I/O");

    assert!(err.to_string().contains("manifest-digest"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_import_non_dry_run_remains_fail_closed() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.mode = "explicit-import";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("explicit import execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_merge_non_dry_run_remains_fail_closed() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.mode = "explicit-merge";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("explicit merge execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}
