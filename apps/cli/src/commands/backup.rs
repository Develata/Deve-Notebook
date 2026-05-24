//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-locator-contract
//!   - 18_backup#backup-provider-dispatch-contract
//!   - 18_backup#backup-secret-ref-contract
//!   - 18_backup#backup-command-output-contract
//!
//! Read-only backup locator and provider adapter command surface.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingStatus, BackupCommandKind, BackupLocator, BackupPlanEffect, BackupPlanInput,
    backup_command_plan, dispatch_backup_provider_adapter, parse_backup_credential_ref,
    parse_backup_key_ref,
};

pub fn inspect(
    locator: &str,
    branch: Option<&str>,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<()> {
    for line in inspect_lines(locator, branch, credential_ref, key_ref)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn inspect_lines(
    locator: &str,
    branch: Option<&str>,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let locator = BackupLocator::parse(locator)?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::InspectBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::InspectOnly,
    })?;
    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!(
            "endpoint={}",
            locator.endpoint.as_deref().unwrap_or("<provider-default>")
        ),
        format!("namespace={}", locator.namespace),
        format!("repo_root_path={}", locator.repo_root_path),
    ];

    if let Some(writer_identity) = branch {
        let branch = locator.branch_locator(writer_identity)?;
        lines.push(format!("branch_writer={}", branch.writer_identity));
        lines.push(format!("branch_path={}", branch.branch_path));
        lines.push(format!("branch_manifest={}", branch.branch_manifest_path()));
        lines.push(format!("pack_prefix={}", branch.pack_prefix()));
    }

    append_provider_adapter_lines(&mut lines, &locator, credential_ref, key_ref)?;

    Ok(lines)
}

fn append_provider_adapter_lines(
    lines: &mut Vec<String>,
    locator: &BackupLocator,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<()> {
    let (Some(credential_ref), Some(key_ref)) = (credential_ref, key_ref) else {
        if credential_ref.is_some() || key_ref.is_some() {
            bail!(
                "backup inspect provider adapter plan requires both --credential-ref and --key-ref"
            );
        }
        return Ok(());
    };

    let adapter =
        dispatch_backup_provider_adapter(deve_core::backup::BackupProviderDispatchInput {
            locator: locator.clone(),
            credential_ref: parse_backup_credential_ref(credential_ref)?,
            key_ref: parse_backup_key_ref(key_ref)?,
        })?;

    lines.push(format!("adapter_provider={}", adapter.provider.protocol()));
    lines.push(format!(
        "credential_ref={}",
        adapter.credential_ref.redacted()
    ));
    lines.push(format!("key_ref={}", adapter.key_ref.redacted()));
    lines.push(format!(
        "provider_metadata_diagnostic_only={}",
        adapter.provider_metadata_is_diagnostic_only
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::inspect_lines;

    #[test]
    fn backup_inspect_prints_sanitized_locator_components() {
        let lines = inspect_lines(
            "s3+https://r2.example.com/bucket-name/deve/",
            Some("writer-1"),
            None,
            None,
        )
        .expect("inspect");

        assert_eq!(lines[0], "backup_locator: provider=s3+https");
        assert!(
            lines
                .iter()
                .any(|line| line == "endpoint=https://r2.example.com")
        );
        assert!(lines.iter().any(|line| line == "namespace=bucket-name"));
        assert!(lines.iter().any(|line| line == "repo_root_path=deve"));
        assert!(
            lines
                .iter()
                .any(|line| line == "branch_path=deve/branches/writer-1")
        );
    }

    #[test]
    fn backup_inspect_can_plan_provider_adapter_with_redacted_refs() {
        let lines = inspect_lines(
            "webdav+https://dav.example.com/notebooks/deve/",
            None,
            Some("env:DEVE_BACKUP_TOKEN"),
            Some("keyring:deve/default-backup-key"),
        )
        .expect("inspect adapter");

        assert!(
            lines
                .iter()
                .any(|line| line == "command=InspectBackupTarget")
        );
        assert!(lines.iter().any(|line| line == "effect=InspectOnly"));
        assert!(
            lines
                .iter()
                .any(|line| line == "adapter_provider=webdav+https")
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "credential_ref=env:<redacted>")
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "key_ref=keyring:<redacted>")
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "provider_metadata_diagnostic_only=true")
        );
    }

    #[test]
    fn backup_inspect_requires_credential_and_key_refs_together() {
        let err = inspect_lines(
            "s3://bucket-name/deve/",
            None,
            Some("env:DEVE_BACKUP_TOKEN"),
            None,
        )
        .expect_err("partial refs must fail closed");

        assert!(err.to_string().contains("requires both"));
    }

    #[test]
    fn backup_inspect_rejects_locator_with_secret_material() {
        let err = inspect_lines(
            "webdav+https://user:pass@dav.example.com/deve/",
            None,
            None,
            None,
        )
        .expect_err("secret material should fail closed");

        assert!(err.to_string().contains("must not contain credentials"));
    }
}
