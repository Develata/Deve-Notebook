//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-command-output-contract
//!
//! Backup command dispatch boundary.

use crate::BackupAction;
use crate::commands;

pub fn run(action: BackupAction) -> anyhow::Result<()> {
    match action {
        BackupAction::Bind {
            locator,
            repo_id,
            branch_name,
            writer,
            local_writer,
            access,
            dry_run,
        } => commands::backup::bind(
            &locator,
            &repo_id,
            &branch_name,
            &writer,
            &local_writer,
            &access,
            dry_run,
        )?,
        BackupAction::Inspect {
            locator,
            branch,
            credential_ref,
            key_ref,
        } => commands::backup::inspect(
            &locator,
            branch.as_deref(),
            credential_ref.as_deref(),
            key_ref.as_deref(),
        )?,
        BackupAction::List { locator, objects } => commands::backup::list(&locator, &objects)?,
        BackupAction::Verify {
            locator,
            branch,
            objects,
            expected_packs,
        } => commands::backup::verify(&locator, &branch, &objects, &expected_packs)?,
        BackupAction::Restore {
            locator,
            repo_id,
            manifest_repo_id,
            branch,
            manifest_digest,
            pack_digests,
            mode,
            write_gate,
            manifest_verified,
            packs_downloaded,
            packs_decrypted,
            dry_run,
        } => commands::backup::restore(commands::backup::RestoreCommandInput {
            locator: &locator,
            repo_id: &repo_id,
            manifest_repo_id: &manifest_repo_id,
            branch: &branch,
            manifest_digest: &manifest_digest,
            pack_digests: &pack_digests,
            mode: &mode,
            write_gate,
            manifest_verified,
            packs_downloaded,
            packs_decrypted,
            dry_run,
        })?,
    }
    Ok(())
}
