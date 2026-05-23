//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-locator-contract
//!
//! Read-only backup locator command surface.

use deve_core::backup::BackupLocator;

pub fn inspect(locator: &str, branch: Option<&str>) -> anyhow::Result<()> {
    for line in inspect_lines(locator, branch)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn inspect_lines(locator: &str, branch: Option<&str>) -> anyhow::Result<Vec<String>> {
    let locator = BackupLocator::parse(locator)?;
    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
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

    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::inspect_lines;

    #[test]
    fn backup_inspect_prints_sanitized_locator_components() {
        let lines = inspect_lines(
            "s3+https://r2.example.com/bucket-name/deve/",
            Some("writer-1"),
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
    fn backup_inspect_rejects_locator_with_secret_material() {
        let err = inspect_lines("webdav+https://user:pass@dav.example.com/deve/", None)
            .expect_err("secret material should fail closed");

        assert!(err.to_string().contains("must not contain credentials"));
    }
}
