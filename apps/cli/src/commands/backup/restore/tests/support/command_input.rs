use super::super::super::RestoreCommandInput;

pub(in crate::commands::backup::restore::tests) const REPO_ID: &str =
    "11111111-1111-1111-1111-111111111111";
pub(in crate::commands::backup::restore::tests) const DIGEST_A: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(in crate::commands::backup::restore::tests) const DIGEST_B: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(in crate::commands::backup::restore::tests) type RestoreFlagSetter =
    fn(&mut RestoreCommandInput<'_>);
pub(in crate::commands::backup::restore::tests) type ForbiddenFlagCase =
    (&'static str, RestoreFlagSetter);

pub(in crate::commands::backup::restore::tests) fn input<'a>(
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
        credential_ref: None,
        key_ref: None,
        pack_sequence: None,
        ledger_start: None,
        ledger_end: None,
        ledger_event_count: None,
        snapshot_count: None,
    }
}

pub(in crate::commands::backup::restore::tests) fn download_input<'a>(
    manifest_digest: &'a str,
    pack_digests: &'a [String],
) -> RestoreCommandInput<'a> {
    let mut command = input(pack_digests, "remote-readonly", false, false);
    command.manifest_digest = manifest_digest;
    command.manifest_verified = false;
    command.packs_downloaded = false;
    command.packs_decrypted = false;
    command.credential_ref = Some("env:DEVE_BACKUP_CREDENTIALS");
    command.key_ref = Some("env:DEVE_BACKUP_KEY");
    command
}
