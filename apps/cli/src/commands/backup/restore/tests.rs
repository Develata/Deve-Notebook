use super::{RestoreCommandInput, restore_lines};

const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";
const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn input<'a>(
    pack_digests: &'a [String],
    mode: &'a str,
    write_gate: bool,
    dry_run: bool,
) -> RestoreCommandInput<'a> {
    RestoreCommandInput {
        locator: "s3://bucket-name/deve/",
        repo_id: REPO_ID,
        manifest_repo_id: REPO_ID,
        branch: "writer-1",
        manifest_digest: DIGEST_A,
        pack_digests,
        mode,
        write_gate,
        manifest_verified: true,
        packs_downloaded: true,
        packs_decrypted: true,
        dry_run,
    }
}

#[test]
fn plans_remote_readonly_restore_flow_without_candidate_admission() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let lines = restore_lines(input(&pack_digests, "remote-readonly", false, true))
        .expect("restore dry-run");

    assert!(lines.iter().any(|line| line == "command=RestoreBackup"));
    assert!(lines.iter().any(|line| line == "effect=RemoteDownload"));
    assert!(lines.iter().any(|line| line == "dry_run=true"));
    assert!(lines.iter().any(|line| line == "artifact_io=false"));
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=PacksDecrypted")
    );
    assert!(lines.iter().any(
        |line| line == "candidate_admission=typed_verification_and_decrypted_evidence_required"
    ));
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

#[test]
fn explicit_import_requires_write_gate() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let err = restore_lines(input(&pack_digests, "explicit-import", false, true))
        .expect_err("explicit import must require gate");
    assert!(err.to_string().contains("write gate"));

    let lines = restore_lines(input(&pack_digests, "explicit-import", true, true))
        .expect("explicit import dry-run");
    assert!(lines.iter().any(|line| line == "effect=ExplicitImport"));
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=true")
    );
}

#[test]
fn fails_closed_on_repo_mismatch_and_incomplete_evidence() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let mut mismatched = input(&pack_digests, "remote-readonly", false, true);
    mismatched.manifest_repo_id = "22222222-2222-2222-2222-222222222222";
    let err = restore_lines(mismatched).expect_err("repo mismatch must fail closed");
    assert!(err.to_string().contains("repo id"));

    let mut incomplete = input(&pack_digests, "remote-readonly", false, true);
    incomplete.packs_decrypted = false;
    let err = restore_lines(incomplete).expect_err("incomplete evidence must fail closed");
    assert!(err.to_string().contains("--packs-decrypted"));
}

#[test]
fn requires_dry_run_and_known_mode() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let err = restore_lines(input(&pack_digests, "remote-readonly", false, false))
        .expect_err("dry-run required");
    assert!(err.to_string().contains("--dry-run"));

    let err =
        restore_lines(input(&pack_digests, "import", false, true)).expect_err("mode rejected");
    assert!(err.to_string().contains("mode must"));
}
