use super::*;
use crate::sync::SyncManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod validation;

fn init_cataloged_repo(
    ledger: &Path,
    projection_base: &Path,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let repo_id = uuid::Uuid::new_v4();
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        ledger,
        8,
        Some(&execution_name),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;
    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, projection_base)?;
    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    crate::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    Ok((repo, repo_id))
}

fn locator_base_from_file(path: &Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    Ok(value["locators"][0]["projection_base_abs"]
        .as_str()
        .expect("projection base")
        .to_string())
}

#[test]
fn set_projection_base_for_all_local_repos_checked_restores_previous_root_when_catalog_refresh_fails()
-> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let first_projection_base = dir.path().join("notes-a");
    let (mut repo, repo_id) = init_cataloged_repo(&ledger, &first_projection_base)?;
    assert_eq!(
        repo.local_repo_workspace_root(&repo_id.to_string())?,
        std::fs::canonicalize(&first_projection_base)?.join(repo_id.to_string())
    );

    std::fs::remove_dir_all(ledger.join("local"))?;

    let second_projection_base = dir.path().join("notes-b");
    let err = repo
        .set_projection_base_for_all_local_repos_checked(&second_projection_base)
        .expect_err("catalog refresh failure must fail closed");
    assert!(err.to_string().contains("local"));

    assert_eq!(
        locator_base_from_file(&repo.projection_locator_path())?,
        std::fs::canonicalize(&first_projection_base)?
            .to_string_lossy()
            .to_string()
    );
    Ok(())
}

#[test]
fn projection_locator_toml_roundtrip() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;

    let locators = repo.list_projection_locators()?;
    assert_eq!(locators.len(), 1);
    assert_eq!(locators[0].repo_id, repo_id);
    assert_eq!(locators[0].workspace_segment, repo_id.to_string());
    assert_eq!(
        locators[0].projection_base_abs,
        std::fs::canonicalize(&base)?
    );

    let reopened = read_projection_locator_file(&repo.projection_locator_path())?;
    assert_eq!(reopened.version, 2);
    assert_eq!(reopened.locators, locators);
    assert_eq!(
        projection_locator_record_for_repo_id(&ledger, repo_id)?,
        Some(locators[0].clone())
    );
    Ok(())
}

#[test]
fn projection_locator_missing_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    repo.remove_projection_locator_for_repo_id(repo_id)?;

    let err = repo
        .local_repo_workspace_root(&repo_id.to_string())
        .expect_err("missing locator must fail closed");

    assert!(err.to_string().contains("Projection Locator missing"));
    Ok(())
}

#[test]
fn projection_locator_invalid_init_repo_name_has_no_side_effects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger = dir.path().join("ledger");

    let err = match RepoManager::init(&ledger, 8, Some("../evil"), Some("urn:evil")) {
        Ok(_) => panic!("invalid init repo name must fail before touching disk"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("single safe path segment"));
    assert!(!ledger.exists(), "invalid init must not create ledger dir");
}

#[test]
fn projection_locator_custom_base_creates_repo_workspace_notegit() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("my-notebooks");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    let repo = Arc::new(repo);

    SyncManager::new_checked(repo.clone())?.scan()?;

    let workspace = repo.local_repo_workspace_root(&repo_id.to_string())?;
    assert!(workspace.join(".notegit").is_dir());
    crate::utils::notegit::validate_repo_identity_marker(&workspace, repo_id)?;
    assert!(workspace.join(".gitignore").is_file());
    Ok(())
}

#[test]
fn projection_locator_scan_uses_computed_workspace_only() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("my-notebooks");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    let repo = Arc::new(repo);
    let workspace = repo.ensure_local_repo_workspace_identity(&repo_id.to_string())?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(base.join("a.md"), "base sibling")?;
    std::fs::write(workspace.join("a.md"), "workspace doc")?;

    SyncManager::new_checked(repo.clone())?.scan()?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_id.to_string())?;
    let mut paths = pending
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, vec!["a.md"]);
    Ok(())
}

#[test]
fn projection_locator_check_rejects_identity_marker_mismatch() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    let workspace = repo.local_repo_workspace_root(&repo_id.to_string())?;
    std::fs::remove_dir_all(crate::utils::notegit::repo_dir(&workspace))?;
    crate::utils::notegit::ensure_repo_identity_marker(
        &workspace,
        uuid::Uuid::from_u128(99),
        "foreign",
    )?;

    let err = repo
        .check_projection_locator_for_local_repo(&repo_id.to_string())
        .expect_err("mismatched identity marker must fail closed");

    assert!(err.to_string().contains("identity marker repo_id mismatch"));
    Ok(())
}

#[test]
fn projection_materialize_rejects_nonempty_workspace_missing_identity_marker() -> anyhow::Result<()>
{
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    let repo = Arc::new(repo);
    let workspace = repo.local_repo_workspace_root(&repo_id.to_string())?;
    std::fs::remove_dir_all(crate::utils::notegit::repo_dir(&workspace))?;
    std::fs::write(workspace.join("foreign.md"), "foreign")?;

    let err = SyncManager::new_checked(repo.clone())?
        .scan()
        .expect_err("nonempty workspace without identity marker must fail closed");

    assert!(err.to_string().contains("missing .notegit identity marker"));
    Ok(())
}

#[test]
fn projection_locator_shared_base_allows_distinct_repo_roots() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (main, default_id) = init_cataloged_repo(&ledger, &base)?;
    let (_, wiki_id) = init_cataloged_repo(&ledger, &base)?;

    assert_eq!(
        main.local_repo_workspace_root(&default_id.to_string())?,
        std::fs::canonicalize(&base)?.join(default_id.to_string())
    );
    assert_eq!(
        main.local_repo_workspace_root(&wiki_id.to_string())?,
        std::fs::canonicalize(&base)?.join(wiki_id.to_string())
    );
    Ok(())
}

#[test]
fn projection_locator_set_all_validates_final_map_not_intermediate_mix() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let old_base = dir.path().join("notes");
    let new_base = old_base.join("wiki");
    let (mut main, default_id) = init_cataloged_repo(&ledger, &old_base)?;
    let (_, wiki_id) = init_cataloged_repo(&ledger, &old_base)?;
    main.set_projection_base_for_all_local_repos_checked(&new_base)?;

    let new_base = std::fs::canonicalize(&new_base)?;
    assert_eq!(
        main.local_repo_workspace_root(&default_id.to_string())?,
        new_base.join(default_id.to_string())
    );
    assert_eq!(
        main.local_repo_workspace_root(&wiki_id.to_string())?,
        new_base.join(wiki_id.to_string())
    );
    Ok(())
}

#[test]
fn projection_locator_nested_workspace_roots_fail_closed() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (main, default_id) = init_cataloged_repo(&ledger, &base)?;
    let wiki_base = dir.path().join("wiki-notes");
    let (_, wiki_id) = init_cataloged_repo(&ledger, &wiki_base)?;
    let err = main
        .set_projection_base_for_repo_id(wiki_id, base.join(default_id.to_string()))
        .expect_err("nested workspace roots must fail closed");

    assert!(
        err.to_string()
            .contains("Projection workspace nesting conflict")
    );
    Ok(())
}

#[test]
fn projection_locator_normal_set_rejects_unknown_repo() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let repo_id = uuid::Uuid::new_v4();

    let err = repo
        .set_projection_base_for_repo_id(repo_id, &base)
        .expect_err("unknown repo must fail closed");

    assert!(err.to_string().contains("unknown local repo"));
    Ok(())
}

#[test]
fn projection_locator_rejects_reserved_repo_path_segments() {
    for name in [
        ".",
        "..",
        "CON",
        "nul.txt",
        "COM1",
        "LPT9",
        "bad/name",
        "bad\\name",
        "bad:name",
        "trail ",
        "trail.",
        "has\0nul",
    ] {
        let err = safe_repo_path_segment(name).expect_err("repo name must fail closed");
        assert!(
            !err.to_string().is_empty(),
            "invalid repo name must produce a diagnostic: {name:?}"
        );
    }
}

#[test]
fn projection_locator_segment_is_immutable_and_independent_of_host_alias() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    repo.host_repo_alias_runtime()
        .set_alias(repo_id, "paper.redb", 0)?;

    assert_eq!(
        repo.local_repo_workspace_root(&repo_id.to_string())?,
        std::fs::canonicalize(&base)?.join(repo_id.to_string())
    );
    assert_eq!(
        repo.projection_locator_for_local_repo(&repo_id.to_string())?
            .workspace_segment,
        repo_id.to_string()
    );
    Ok(())
}

#[test]
fn projection_locator_normalized_workspace_key_detects_case_and_unicode_conflicts() {
    let base = PathBuf::from("notes");
    let id = uuid::Uuid::from_u128(1);

    assert_eq!(
        normalized_workspace_key(&base, "Default"),
        normalized_workspace_key(&base, "default")
    );
    assert_eq!(
        normalized_workspace_key(&base, "\u{00e9}"),
        normalized_workspace_key(&base, "e\u{0301}")
    );
    assert_ne!(
        normalized_workspace_key(&base, &id.to_string()),
        normalized_workspace_key(&base, &uuid::Uuid::from_u128(2).to_string())
    );
}

#[test]
fn projection_locator_rejects_protected_workspace_roots() -> anyhow::Result<()> {
    for protected_base in ["ledger/.host", ".git", ".notegit"] {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let initial_base = dir.path().join("initial-notes");
        let (repo, repo_id) = init_cataloged_repo(&ledger, &initial_base)?;
        let base = dir.path().join(protected_base);

        let err = repo
            .set_projection_base_for_repo_id(repo_id, &base)
            .expect_err("protected workspace root must fail closed");
        let message = err.to_string();
        assert!(
            message.contains("ledger_dir")
                || message.contains(".notegit")
                || message.contains(".git"),
            "unexpected protected root diagnostic: {message}"
        );
    }
    Ok(())
}

#[test]
fn projection_locator_workspace_deveignore_filters_startup_scan() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (repo, repo_id) = init_cataloged_repo(&ledger, &base)?;
    let repo = Arc::new(repo);
    let workspace = repo.ensure_local_repo_workspace_identity(&repo_id.to_string())?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(workspace.join(".deveignore"), "ignored.md\n")?;
    std::fs::write(workspace.join("ignored.md"), "ignored")?;
    std::fs::write(workspace.join("kept.md"), "kept")?;

    SyncManager::new_checked(repo.clone())?.scan()?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_id.to_string())?;
    let mut paths = pending
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, vec!["kept.md"]);
    Ok(())
}
