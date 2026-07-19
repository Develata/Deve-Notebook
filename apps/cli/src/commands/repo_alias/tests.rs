//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 14_commands#repo-alias-command-contract

use super::*;
use deve_core::ledger::{
    HostRepoAliasImportWarning, HostRepoAliasImportWarningReason, RepoManager,
};
use serde_json::json;

#[test]
fn export_and_import_commands_roundtrip_alias_json() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("default"), None)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    drop(repo);

    set(&ledger, repo_id, "math", 0)?;
    let output = dir.path().join("aliases.json");
    export(&ledger, &output)?;
    let exported: serde_json::Value = serde_json::from_slice(&std::fs::read(&output)?)?;
    assert_eq!(exported["aliases"][0]["repo_id"], repo_id.to_string());
    assert_eq!(exported["aliases"][0]["alias"], "math");
    assert!(exported["aliases"][0].get("alias_revision").is_none());

    let input = dir.path().join("import.json");
    std::fs::write(
        &input,
        serde_json::to_vec(&json!({
            "format": "deve.host-repo-aliases",
            "version": 1,
            "aliases": [{"repo_id": repo_id, "alias": "algebra"}],
        }))?,
    )?;
    import_aliases(&ledger, &input, false)?;
    let reopened = RepoManager::init(&ledger, 8, None, None)?;
    assert_eq!(
        reopened.host_repo_alias_runtime().binding(repo_id)?.alias,
        "math"
    );
    drop(reopened);
    import_aliases(&ledger, &input, true)?;
    let reopened = RepoManager::init(&ledger, 8, None, None)?;
    assert_eq!(
        reopened.host_repo_alias_runtime().binding(repo_id)?.alias,
        "algebra"
    );
    Ok(())
}

#[test]
fn bounded_reader_rejects_oversized_input() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("large.json");
    std::fs::write(&input, vec![b'x'; HOST_REPO_ALIAS_IMPORT_MAX_BYTES + 1])?;
    assert!(read_bounded_input(&input).is_err());
    Ok(())
}

#[test]
fn bounded_reader_rejects_a_symlink_without_reading_its_target() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("target.json");
    let link = dir.path().join("input.json");
    std::fs::write(&target, b"secret")?;
    if !create_file_symlink_or_skip(&target, &link)? {
        return Ok(());
    }
    assert!(read_bounded_input(&link).is_err());
    assert_eq!(std::fs::read(&target)?, b"secret");
    Ok(())
}

#[test]
fn alias_commands_never_create_an_empty_ledger() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("missing-ledger");
    let output = dir.path().join("aliases.json");
    assert!(export(&ledger, &output).is_err());
    assert!(!ledger.exists());
    assert!(!output.exists());
    assert!(set(&ledger, RepoId::new_v4(), "alias", 0).is_err());
    assert!(!ledger.exists());
    Ok(())
}

#[test]
fn alias_commands_open_a_non_default_repo_without_creating_another_repo() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("research"), None)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    drop(repo);
    set(&ledger, repo_id, "math", 0)?;
    let entries = std::fs::read_dir(ledger.join("local"))?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        HostRepoAliasRuntime::open_existing(&ledger)?
            .binding(repo_id)?
            .alias,
        "math"
    );
    Ok(())
}

#[test]
fn export_is_no_clobber_and_rejects_the_ledger_tree() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("default"), None)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    repo.host_repo_alias_runtime()
        .set_alias(repo_id, "math", 0)?;
    drop(repo);
    let output = dir.path().join("aliases.json");
    std::fs::write(&output, b"keep")?;
    assert!(export(&ledger, &output).is_err());
    assert_eq!(std::fs::read(&output)?, b"keep");
    assert!(export(&ledger, &ledger.join("aliases.json")).is_err());
    assert!(!ledger.join("aliases.json").exists());
    Ok(())
}

#[test]
fn warning_summary_uses_stdout_for_counts_and_stderr_for_every_reason() {
    let repo_id = RepoId::new_v4();
    let summary = HostRepoAliasImportSummary {
        accepted: 1,
        changed: 1,
        unchanged: 0,
        skipped: 2,
        warnings: vec![
            HostRepoAliasImportWarning {
                index: 1,
                repo_id: Some(repo_id),
                reason: HostRepoAliasImportWarningReason::UnknownLocalRepo,
            },
            HostRepoAliasImportWarning {
                index: 2,
                repo_id: Some(repo_id),
                reason: HostRepoAliasImportWarningReason::AliasTooLong,
            },
        ],
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    write_summary(&summary, true, &mut stdout, &mut stderr).expect("write summary");
    let stdout = String::from_utf8(stdout).expect("stdout UTF-8");
    let stderr = String::from_utf8(stderr).expect("stderr UTF-8");
    assert!(stdout.contains("mode=apply accepted=1 changed=1 unchanged=0 skipped=2"));
    assert!(!stdout.contains("warning:"));
    assert!(stderr.contains("repo_id is not an active local repository"));
    assert!(stderr.contains("alias exceeds 256 UTF-8 bytes"));
    assert_eq!(stderr.lines().count(), 2);
}

#[cfg(unix)]
fn create_file_symlink_or_skip(
    target: &std::path::Path,
    link: &std::path::Path,
) -> std::io::Result<bool> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(true)
}

#[cfg(windows)]
fn create_file_symlink_or_skip(
    target: &std::path::Path,
    link: &std::path::Path,
) -> std::io::Result<bool> {
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(not(any(unix, windows)))]
fn create_file_symlink_or_skip(
    _target: &std::path::Path,
    _link: &std::path::Path,
) -> std::io::Result<bool> {
    Ok(false)
}
