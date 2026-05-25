//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-command-output-contract
//!
//! Backup CLI action shape.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum BackupAction {
    /// Plan a backup branch binding without persisting binding state
    Bind {
        #[arg(long)]
        locator: String,
        #[arg(long = "repo-id")]
        repo_id: String,
        #[arg(long = "branch-name")]
        branch_name: String,
        #[arg(long)]
        writer: String,
        #[arg(long = "local-writer")]
        local_writer: String,
        #[arg(long)]
        access: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Parse and inspect a backup locator without binding it to repo authority
    Inspect {
        #[arg(long)]
        locator: String,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long = "credential-ref")]
        credential_ref: Option<String>,
        #[arg(long = "key-ref")]
        key_ref: Option<String>,
    },
    /// List backup branch manifests from explicit remote object paths
    List {
        #[arg(long)]
        locator: String,
        #[arg(long = "object")]
        objects: Vec<String>,
    },
    /// Verify expected backup remote layout from explicit remote object paths
    Verify {
        #[arg(long)]
        locator: String,
        #[arg(long)]
        branch: String,
        #[arg(long = "object")]
        objects: Vec<String>,
        #[arg(long = "pack")]
        expected_packs: Vec<String>,
    },
    /// Plan a branch backup upload without provider IO
    Run {
        #[arg(long)]
        locator: String,
        #[arg(long = "repo-id")]
        repo_id: String,
        #[arg(long = "branch-name")]
        branch_name: String,
        #[arg(long)]
        writer: String,
        #[arg(long = "local-writer")]
        local_writer: String,
        #[arg(long = "credential-ref")]
        credential_ref: String,
        #[arg(long = "key-ref")]
        key_ref: String,
        #[arg(long = "pack-sequence", default_value_t = 1)]
        pack_sequence: u64,
        #[arg(long = "ledger-start")]
        ledger_start: Option<u64>,
        #[arg(long = "ledger-end")]
        ledger_end: Option<u64>,
        #[arg(long = "ledger-events", default_value_t = 1)]
        ledger_event_count: u64,
        #[arg(long = "snapshot-count", default_value_t = 0)]
        snapshot_count: u64,
        #[arg(long = "payload-digest")]
        payload_digest: String,
        #[arg(long)]
        encrypted: bool,
        #[arg(long)]
        authenticated: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Plan restore candidate admission from verified backup metadata
    Restore {
        #[arg(long)]
        locator: String,
        #[arg(long = "repo-id")]
        repo_id: String,
        #[arg(long = "manifest-repo-id")]
        manifest_repo_id: String,
        #[arg(long)]
        branch: String,
        #[arg(long = "manifest-digest")]
        manifest_digest: String,
        #[arg(long = "pack-digest")]
        pack_digests: Vec<String>,
        #[arg(long, default_value = "remote-readonly")]
        mode: String,
        #[arg(long = "write-gate")]
        write_gate: bool,
        #[arg(long = "manifest-verified")]
        manifest_verified: bool,
        #[arg(long = "packs-downloaded")]
        packs_downloaded: bool,
        #[arg(long = "packs-decrypted")]
        packs_decrypted: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Plan backup branch binding removal without persisting mutation
    Unbind {
        #[arg(long)]
        locator: String,
        #[arg(long = "repo-id")]
        repo_id: String,
        #[arg(long = "branch-name")]
        branch_name: String,
        #[arg(long)]
        writer: String,
        #[arg(long = "local-writer")]
        local_writer: String,
        #[arg(long)]
        access: String,
        #[arg(long)]
        dry_run: bool,
    },
}
