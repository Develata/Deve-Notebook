use super::*;

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
