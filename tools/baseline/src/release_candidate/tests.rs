use super::run;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn artifact_role_keys_match_manifest_serialization() {
    for role in super::manifest::ArtifactRole::ALL {
        let serialized = serde_json::to_value(role).expect("serialize artifact role");
        assert_eq!(serialized.as_str(), Some(role.key()));
    }
}

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "deve-release-candidate-{label}-{}-{nonce}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test candidate directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn assembles_and_verifies_exact_candidate() {
    let fixture = fixture("valid");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    run(&arguments(fixture.path(), "verify")).expect("verify candidate");

    let manifest =
        fs::read_to_string(fixture.path().join("release-candidate.json")).expect("read manifest");
    assert!(manifest.ends_with('\n'));
    assert!(manifest.contains("\"schema\": 1"));
    assert!(fixture.path().join("SHA256SUMS").is_file());
    assert!(
        fixture
            .path()
            .join("release-candidate-SHA256SUMS")
            .is_file()
    );
    let checksums =
        fs::read_to_string(fixture.path().join("SHA256SUMS")).expect("read public checksums");
    assert!(checksums.contains("  deve-notebook-0.1.0-windows-x64.msi\n"));
    assert!(!checksums.contains("  artifacts/windows/"));
}

#[test]
fn verify_rejects_artifact_corruption() {
    let fixture = fixture("corruption");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    fs::write(
        fixture
            .path()
            .join("artifacts/android/deve-notebook-0.1.0-android-arm64.apk"),
        b"corrupt",
    )
    .expect("corrupt APK");

    let error = run(&arguments(fixture.path(), "verify")).expect_err("must reject corruption");
    assert!(
        format!("{error:#}").contains("artifact records do not match"),
        "{error:#}"
    );
}

#[test]
fn verify_rejects_extra_file() {
    let fixture = fixture("extra");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    fs::write(fixture.path().join("untracked.txt"), b"unexpected").expect("write extra");

    let error = run(&arguments(fixture.path(), "verify")).expect_err("must reject extra file");
    assert!(format!("{error:#}").contains("extra=[\"untracked.txt\"]"));
}

#[test]
fn verify_rejects_head_mismatch() {
    let fixture = fixture("head-mismatch");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    let mut args = arguments(fixture.path(), "verify");
    replace_value(&mut args, "--head", &"d".repeat(40));

    let error = run(&args).expect_err("must reject other HEAD");
    assert!(format!("{error:#}").contains("HEAD does not match"));
}

#[test]
fn verify_rejects_version_mismatch() {
    let fixture = fixture("version-mismatch");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    let mut args = arguments(fixture.path(), "verify");
    replace_value(&mut args, "--version", "0.2.0");

    let error = run(&args).expect_err("must reject other version");
    assert!(format!("{error:#}").contains("version does not match"));
}

#[test]
fn assemble_rejects_parent_path_escape() {
    let fixture = fixture("parent-escape");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(&mut args, "--windows-msi", "../outside.msi");

    let error = run(&args).expect_err("must reject parent path");
    assert!(format!("{error:#}").contains("non-canonical segment"));
}

#[test]
fn assemble_rejects_windows_absolute_path_on_every_host() {
    let fixture = fixture("drive-escape");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(&mut args, "--windows-msi", "C:/outside.msi");

    let error = run(&args).expect_err("must reject Windows drive path");
    assert!(format!("{error:#}").contains("canonical forward-slash relative path"));
}

#[test]
fn assemble_rejects_non_frozen_windows_path() {
    let fixture = fixture("non-frozen-windows");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(
        &mut args,
        "--windows-msi",
        "other/deve-notebook-0.1.0-windows-x64.msi",
    );

    let error = run(&args).expect_err("must reject non-frozen Windows path");
    assert!(format!("{error:#}").contains("does not match release freeze"));
}

#[test]
fn assemble_rejects_non_frozen_macos_architecture() {
    let fixture = fixture("non-frozen-macos");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(
        &mut args,
        "--macos-dmg",
        "artifacts/macos/deve-notebook-0.1.0-macos-riscv64.dmg",
    );

    let error = run(&args).expect_err("must reject non-frozen macOS path");
    assert!(format!("{error:#}").contains("host-architecture one-of"));
}

#[test]
fn assemble_rejects_duplicate_release_asset_basename() {
    let fixture = fixture("duplicate-basename");
    fs::create_dir_all(fixture.path().join("other")).expect("create other directory");
    fs::write(
        fixture.path().join("other/provenance.bundle"),
        br#"{"bundle":"duplicate"}"#,
    )
    .expect("write duplicate basename");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(&mut args, "--docker-sbom-bundle", "other/provenance.bundle");

    let error = run(&args).expect_err("must reject duplicate basenames");
    assert!(format!("{error:#}").contains("unique case-insensitive basenames"));
}

#[test]
fn assemble_rejects_reserved_control_basename_below_subdirectory() {
    let fixture = fixture("reserved-control-basename");
    fs::create_dir_all(fixture.path().join("other")).expect("create other directory");
    fs::write(
        fixture.path().join("other/SHA256SUMS"),
        br#"{"bundle":"reserved"}"#,
    )
    .expect("write reserved basename");
    let mut args = arguments(fixture.path(), "assemble");
    replace_value(&mut args, "--docker-sbom-bundle", "other/SHA256SUMS");

    let error = run(&args).expect_err("must reject generated basename collision");
    assert!(
        format!("{error:#}").contains("basename collides"),
        "{error:#}"
    );
}

#[test]
fn assemble_rejects_symlinked_artifact() {
    let fixture = fixture("symlink");
    let artifact = fixture
        .path()
        .join("artifacts/windows/deve-notebook-0.1.0-windows-x64.msi");
    fs::remove_file(&artifact).expect("remove real MSI");
    let target = fixture.path().join("outside.msi");
    fs::write(&target, b"outside").expect("write target");
    if !create_file_symlink(&target, &artifact) {
        return;
    }

    let error = run(&arguments(fixture.path(), "assemble")).expect_err("must reject symlink");
    assert!(format!("{error:#}").contains("symlink or reparse point"));
}

#[test]
fn assemble_accepts_github_jsonl_attestation_bundle() {
    let fixture = fixture("jsonl-attestation");
    fs::write(
        fixture.path().join("attestations/provenance.bundle"),
        b"{\"bundle\":1}\n{\"bundle\":2}\n",
    )
    .expect("write JSONL attestation");
    run(&arguments(fixture.path(), "assemble")).expect("assemble JSONL candidate");
}

#[test]
fn assemble_rejects_oversized_attestation_before_json_parse() {
    let fixture = fixture("oversized-attestation");
    let path = fixture.path().join("attestations/provenance.bundle");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open attestation")
        .set_len(16 * 1024 * 1024 + 1)
        .expect("grow sparse attestation");

    let error =
        run(&arguments(fixture.path(), "assemble")).expect_err("must reject oversized attestation");
    assert!(format!("{error:#}").contains("resource limit"), "{error:#}");
}

#[test]
fn verify_rejects_oversized_manifest_before_parse() {
    let fixture = fixture("oversized-manifest");
    run(&arguments(fixture.path(), "assemble")).expect("assemble candidate");
    fs::OpenOptions::new()
        .write(true)
        .open(fixture.path().join("release-candidate.json"))
        .expect("open manifest")
        .set_len(1024 * 1024 + 1)
        .expect("grow sparse manifest");

    let error =
        run(&arguments(fixture.path(), "verify")).expect_err("must reject oversized manifest");
    assert!(format!("{error:#}").contains("exceeds"), "{error:#}");
}

#[test]
fn assemble_rejects_minimal_spdx_label_without_document_shape() {
    let fixture = fixture("invalid-spdx-shape");
    fs::write(
        fixture.path().join("sbom/source.spdx.json"),
        br#"{"spdxVersion":"SPDX-2.3"}"#,
    )
    .expect("write invalid SPDX");

    let error = run(&arguments(fixture.path(), "assemble")).expect_err("must reject fake SPDX");
    assert!(format!("{error:#}").contains("dataLicense"), "{error:#}");
}

#[test]
fn assemble_rejects_spdx_arrays_with_non_object_entries() {
    let fixture = fixture("invalid-spdx-array");
    fs::write(
        fixture.path().join("sbom/source.spdx.json"),
        br#"{"spdxVersion":"SPDX-2.3","dataLicense":"CC0-1.0","SPDXID":"SPDXRef-DOCUMENT","name":"source","documentNamespace":"https://example.invalid/source","creationInfo":{"created":"2026-07-15T00:00:00Z","creators":["Tool: test"]},"packages":[null],"relationships":[null]}"#,
    )
    .expect("write malformed SPDX arrays");

    let error = run(&arguments(fixture.path(), "assemble"))
        .expect_err("must reject non-object SPDX entries");
    assert!(
        format!("{error:#}").contains("packages or files"),
        "{error:#}"
    );
}

#[test]
fn assemble_rejects_unexpected_empty_directory() {
    let fixture = fixture("unexpected-empty-directory");
    fs::create_dir(fixture.path().join("extra-empty")).expect("create unexpected directory");

    let error = run(&arguments(fixture.path(), "assemble"))
        .expect_err("must reject unexpected empty directory");
    assert!(format!("{error:#}").contains("directories"), "{error:#}");
}

#[test]
fn preexisting_control_entry_is_rejected_before_assembly() {
    let fixture = fixture("assembly-rollback");
    fs::create_dir(fixture.path().join("SHA256SUMS")).expect("block checksum file creation");
    let error = run(&arguments(fixture.path(), "assemble")).expect_err("assembly must fail");
    assert!(format!("{error:#}").contains("extra_directories"));
    assert!(!fixture.path().join("release-candidate.json").exists());
    assert!(!fixture.path().join("release-candidate-SHA256SUMS").exists());
}

fn fixture(label: &str) -> TestDir {
    let fixture = TestDir::new(label);
    let artifacts = fixture.path().join("artifacts");
    for directory in [
        artifacts.join("windows"),
        artifacts.join("macos"),
        artifacts.join("android"),
        artifacts.join("docker"),
        fixture.path().join("sbom"),
        fixture.path().join("attestations"),
    ] {
        fs::create_dir_all(directory).expect("create candidate directory");
    }
    fs::write(
        artifacts
            .join("windows")
            .join("deve-notebook-0.1.0-windows-x64.msi"),
        b"msi",
    )
    .expect("write MSI");
    fs::write(
        artifacts
            .join("windows")
            .join("deve-notebook-0.1.0-windows-x64-setup.exe"),
        b"nsis",
    )
    .expect("write NSIS");
    fs::write(
        artifacts
            .join("macos")
            .join("deve-notebook-0.1.0-macos-x64.dmg"),
        b"dmg",
    )
    .expect("write DMG");
    fs::write(
        artifacts
            .join("android")
            .join("deve-notebook-0.1.0-android-arm64.apk"),
        b"signed-apk",
    )
    .expect("write APK");
    fs::write(
        artifacts
            .join("docker")
            .join("deve-notebook-0.1.0-linux-amd64.tar"),
        b"docker-archive",
    )
    .expect("write Docker archive");
    fs::write(
        fixture.path().join("sbom/source.spdx.json"),
        spdx_fixture("source"),
    )
    .expect("write source SPDX");
    fs::write(
        fixture.path().join("sbom/docker-image.spdx.json"),
        spdx_fixture("image"),
    )
    .expect("write image SPDX");
    fs::write(
        fixture.path().join("attestations/provenance.bundle"),
        br#"{"bundle":"fixture"}"#,
    )
    .expect("write attestation");
    fs::write(
        fixture.path().join("attestations/docker-sbom.bundle"),
        br#"{"bundle":"docker-sbom-fixture"}"#,
    )
    .expect("write Docker SBOM attestation");
    fixture
}

fn spdx_fixture(name: &str) -> Vec<u8> {
    format!(
        r#"{{"spdxVersion":"SPDX-2.3","dataLicense":"CC0-1.0","SPDXID":"SPDXRef-DOCUMENT","name":"{name}","documentNamespace":"https://example.invalid/{name}","creationInfo":{{"created":"2026-07-15T00:00:00Z","creators":["Tool: test"]}},"packages":[{{"name":"fixture","SPDXID":"SPDXRef-Package"}}],"relationships":[{{"spdxElementId":"SPDXRef-DOCUMENT","relationshipType":"DESCRIBES","relatedSpdxElement":"SPDXRef-Package"}}]}}"#
    )
    .into_bytes()
}

fn arguments(root: &Path, action: &str) -> Vec<String> {
    [
        action.to_owned(),
        "--candidate-dir".into(),
        root.display().to_string(),
        "--output".into(),
        "release-candidate.json".into(),
        "--head".into(),
        "c".repeat(40),
        "--version".into(),
        "0.1.0".into(),
        "--workflow-path".into(),
        ".github/workflows/release-candidate.yml".into(),
        "--run-id".into(),
        "42".into(),
        "--run-attempt".into(),
        "3".into(),
        "--docker-image-id".into(),
        format!("sha256:{}", "a".repeat(64)),
        "--android-signer-sha256".into(),
        format!("sha256:{}", "b".repeat(64)),
        "--windows-msi".into(),
        "artifacts/windows/deve-notebook-0.1.0-windows-x64.msi".into(),
        "--windows-nsis".into(),
        "artifacts/windows/deve-notebook-0.1.0-windows-x64-setup.exe".into(),
        "--macos-dmg".into(),
        "artifacts/macos/deve-notebook-0.1.0-macos-x64.dmg".into(),
        "--android-apk".into(),
        "artifacts/android/deve-notebook-0.1.0-android-arm64.apk".into(),
        "--docker-archive".into(),
        "artifacts/docker/deve-notebook-0.1.0-linux-amd64.tar".into(),
        "--source-sbom".into(),
        "sbom/source.spdx.json".into(),
        "--image-sbom".into(),
        "sbom/docker-image.spdx.json".into(),
        "--provenance-bundle".into(),
        "attestations/provenance.bundle".into(),
        "--docker-sbom-bundle".into(),
        "attestations/docker-sbom.bundle".into(),
    ]
    .into_iter()
    .collect()
}

fn replace_value(args: &mut [String], flag: &str, value: &str) {
    let index = args
        .iter()
        .position(|arg| arg == flag)
        .expect("flag exists");
    args[index + 1] = value.to_owned();
}

#[cfg(unix)]
fn create_file_symlink(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn create_file_symlink(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}
