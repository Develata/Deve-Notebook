//! plan_ref:
//!   - 04_repository#host-repo-alias-contract

use super::*;

#[test]
fn malformed_container_and_global_budgets_fail_the_whole_import() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let repo = init_alias_repo(&dir.path().join("ledger"))?;
    let runtime = repo.host_repo_alias_runtime();

    assert!(runtime.preview_import_json(b"not-json").is_err());
    assert!(matches!(
        runtime.preview_import_json(
            br#"{"format":"deve.host-repo-aliases","version":2,"aliases":[]}"#
        ),
        Err(HostRepoAliasError::UnsupportedVersion(2))
    ));
    let oversized = vec![b' '; HOST_REPO_ALIAS_IMPORT_MAX_BYTES + 1];
    assert!(matches!(
        runtime.preview_import_json(&oversized),
        Err(HostRepoAliasError::BudgetExceeded {
            budget: "file bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn corrupt_store_fails_closed_without_overwriting_it() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let store_path = crate::utils::notegit::host_dir(&ledger).join("repo-aliases.json");
    std::fs::write(&store_path, b"{broken")?;

    let error = repo
        .host_repo_alias_runtime()
        .set_alias(repo_id, "safe", 0)
        .expect_err("corrupt store must fail closed");
    assert!(matches!(error, HostRepoAliasError::StoreInvalid(_)));
    assert_eq!(std::fs::read(&store_path)?, b"{broken");
    Ok(())
}

#[test]
fn corrupt_catalog_record_is_a_global_failure_and_preserves_alias_store() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    add_alias_repo(&repo, &ledger)?;
    let ids = repo_ids(&repo)?;
    let runtime = repo.host_repo_alias_runtime();
    runtime.set_alias(ids[0], "before", 0)?;
    let store_path = crate::utils::notegit::host_dir(&ledger).join("repo-aliases.json");
    let before = std::fs::read(&store_path)?;
    std::fs::write(
        crate::utils::notegit::host_dir(&ledger)
            .join("repo-catalog")
            .join(format!("{}.json", ids[1])),
        b"not-json",
    )?;
    let input = import_document(vec![
        json!({"repo_id": ids[0], "alias": "after"}),
        json!({"repo_id": ids[1], "alias": "second"}),
    ]);

    assert!(matches!(
        runtime.apply_import_json(&input),
        Err(HostRepoAliasError::Runtime(_))
    ));
    assert_eq!(std::fs::read(&store_path)?, before);
    Ok(())
}

#[test]
fn oversized_catalog_record_is_a_global_failure_and_preserves_alias_store() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let runtime = repo.host_repo_alias_runtime();
    runtime.set_alias(repo_id, "before", 0)?;
    let store_path = crate::utils::notegit::host_dir(&ledger).join("repo-aliases.json");
    let before = std::fs::read(&store_path)?;
    std::fs::write(
        crate::utils::notegit::host_dir(&ledger)
            .join("repo-catalog")
            .join(format!("{repo_id}.json")),
        vec![b'x'; 16 * 1024 + 1],
    )?;
    let input = import_document(vec![json!({"repo_id": repo_id, "alias": "after"})]);

    assert!(matches!(
        runtime.apply_import_json(&input),
        Err(HostRepoAliasError::Runtime(_))
    ));
    assert_eq!(std::fs::read(&store_path)?, before);
    Ok(())
}

#[test]
fn unregistered_repo_file_is_an_unknown_repo_warning_not_a_whole_import_failure()
-> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let good = repo.get_repo_info()?.expect("repo info").uuid;
    let broken = RepoId::new_v4();
    std::fs::write(
        ledger.join("local").join(format!("{broken}.redb")),
        b"not-redb",
    )?;
    let input = import_document(vec![
        json!({"repo_id": broken, "alias": "broken"}),
        json!({"repo_id": good, "alias": "good"}),
    ]);

    let summary = repo.host_repo_alias_runtime().apply_import_json(&input)?;
    assert_eq!(summary.accepted, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.warnings[0].repo_id, Some(broken));
    assert_eq!(
        summary.warnings[0].reason,
        HostRepoAliasImportWarningReason::UnknownLocalRepo
    );
    assert_eq!(repo.host_repo_alias_runtime().binding(good)?.alias, "good");
    Ok(())
}

#[test]
fn unregistered_cached_repo_is_still_an_unknown_repo_warning() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = init_alias_repo(&ledger)?;
    let good = repo.get_repo_info()?.expect("repo info").uuid;
    let broken = RepoId::new_v4();
    let broken_path = ledger.join("local").join(format!("{broken}.redb"));
    let broken_db = std::sync::Arc::new(redb::Database::create(&broken_path)?);
    crate::ledger::database_cache::register_database(&broken_path, broken_db)?;
    let input = import_document(vec![
        json!({"repo_id": broken, "alias": "broken"}),
        json!({"repo_id": good, "alias": "good"}),
    ]);

    let summary = repo.host_repo_alias_runtime().apply_import_json(&input)?;
    assert_eq!(summary.accepted, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(
        summary.warnings[0].reason,
        HostRepoAliasImportWarningReason::UnknownLocalRepo
    );
    assert_eq!(repo.host_repo_alias_runtime().binding(good)?.alias, "good");
    crate::ledger::database_cache::evict_database_paths_under(&ledger)?;
    Ok(())
}

#[test]
fn alias_store_and_lock_symlinks_are_rejected_without_touching_targets() -> anyhow::Result<()> {
    for (link_name, victim_name) in [
        ("repo-aliases.json", "victim-store.txt"),
        ("repo-aliases.lock", "victim-lock.txt"),
    ] {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let repo = init_alias_repo(&ledger)?;
        let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
        let victim = dir.path().join(victim_name);
        std::fs::write(&victim, b"do-not-touch")?;
        let link = crate::utils::notegit::host_dir(&ledger).join(link_name);
        if !create_file_symlink_or_skip(&victim, &link)? {
            return Ok(());
        }
        assert!(
            repo.host_repo_alias_runtime()
                .set_alias(repo_id, "unsafe", 0)
                .is_err()
        );
        assert_eq!(std::fs::read(&victim)?, b"do-not-touch");
    }
    Ok(())
}

#[test]
fn all_import_budget_dimensions_fail_closed() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let repo = init_alias_repo(&dir.path().join("ledger"))?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let runtime = repo.host_repo_alias_runtime();
    let too_many = import_document(
        (0..4097)
            .map(|_| json!({"repo_id": repo_id, "alias": "x"}))
            .collect(),
    );
    assert!(matches!(
        runtime.preview_import_json(&too_many),
        Err(HostRepoAliasError::BudgetExceeded {
            budget: "entry count",
            ..
        })
    ));
    let long_id = import_document(vec![json!({"repo_id": "x".repeat(65), "alias": "x"})]);
    assert!(matches!(
        runtime.preview_import_json(&long_id),
        Err(HostRepoAliasError::BudgetExceeded {
            budget: "repo_id bytes",
            ..
        })
    ));
    let alias = "x".repeat(256);
    let too_many_alias_bytes = import_document(
        (0..2049)
            .map(|_| json!({"repo_id": repo_id, "alias": alias}))
            .collect(),
    );
    assert!(matches!(
        runtime.preview_import_json(&too_many_alias_bytes),
        Err(HostRepoAliasError::BudgetExceeded {
            budget: "total alias bytes",
            ..
        })
    ));
    Ok(())
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
