use super::*;
use crate::sync::SyncManager;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn projection_locator_toml_roundtrip() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;

    repo.set_projection_base_for_local_repo("default", &base)?;

    let locators = repo.list_projection_locators()?;
    assert_eq!(locators.len(), 1);
    assert_eq!(locators[0].repo_id, repo_id);
    assert_eq!(locators[0].repo_name_hint, "default");
    assert_eq!(
        locators[0].projection_base_abs,
        std::fs::canonicalize(&base)?
    );

    let reopened = read_projection_locator_file(&repo.projection_locator_path())?;
    assert_eq!(reopened.version, 1);
    assert_eq!(reopened.locators, locators);
    Ok(())
}

#[test]
fn projection_locator_missing_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;

    let err = repo
        .local_repo_workspace_root("default")
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
    let repo = Arc::new(RepoManager::init(
        &ledger,
        8,
        Some("default"),
        Some("urn:default"),
    )?);
    repo.set_projection_base_for_local_repo("default", &base)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;

    SyncManager::new_checked(repo.clone())?.scan()?;

    let workspace = repo.local_repo_workspace_root("default")?;
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
    let repo = Arc::new(RepoManager::init(
        &ledger,
        8,
        Some("default"),
        Some("urn:default"),
    )?);
    repo.set_projection_base_for_local_repo("default", &base)?;
    let workspace = repo.ensure_local_repo_workspace_identity("default")?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(base.join("a.md"), "base sibling")?;
    std::fs::write(workspace.join("a.md"), "workspace doc")?;

    SyncManager::new_checked(repo.clone())?.scan()?;

    let pending = repo.list_pending_fs_in_local_repo("default")?;
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
    let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_local_repo("default", &base)?;
    let workspace = repo.local_repo_workspace_root("default")?;
    std::fs::create_dir_all(crate::utils::notegit::repo_dir(&workspace))?;
    crate::utils::notegit::ensure_repo_identity_marker(
        &workspace,
        uuid::Uuid::from_u128(99),
        "foreign",
    )?;

    let err = repo
        .check_projection_locator_for_local_repo("default")
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
    let repo = Arc::new(RepoManager::init(
        &ledger,
        8,
        Some("default"),
        Some("urn:default"),
    )?);
    repo.set_projection_base_for_local_repo("default", &base)?;
    let workspace = repo.local_repo_workspace_root("default")?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(workspace.join("foreign.md"), "foreign")?;

    let err = SyncManager::new_checked(repo.clone())?
        .scan()
        .expect_err("nonempty workspace without identity marker must fail closed");

    assert!(err.to_string().contains("missing .notegit identity marker"));
    Ok(())
}

#[test]
fn projection_locator_runtime_check_validates_full_map_conflicts() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;
    let main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let wiki = crate::test_support::create_initialized_local_repo(&ledger, "wiki", "urn:wiki");
    let default_id = main.get_repo_info()?.expect("default repo").uuid;
    let nested_base = base.join(repo_workspace_segment("default", default_id)?);
    std::fs::create_dir_all(&nested_base)?;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 1,
            locators: vec![
                ProjectionLocatorRecord {
                    repo_id: default_id,
                    repo_name_hint: "default".into(),
                    projection_base_abs: std::fs::canonicalize(&base)?,
                    canonicalized_at_unix_ms: 1,
                },
                ProjectionLocatorRecord {
                    repo_id: wiki.uuid,
                    repo_name_hint: "wiki".into(),
                    projection_base_abs: std::fs::canonicalize(&nested_base)?,
                    canonicalized_at_unix_ms: 1,
                },
            ],
        },
    )?;

    let err = main
        .check_projection_locator_for_local_repo("default")
        .expect_err("full locator map nesting conflict must fail");
    assert!(
        err.to_string()
            .contains("Projection workspace nesting conflict")
    );

    let sync_err = match SyncManager::new_checked(Arc::new(main)) {
        Ok(_) => panic!("SyncManager must reject corrupted locator map"),
        Err(err) => err,
    };
    assert!(
        sync_err
            .to_string()
            .contains("Projection workspace nesting conflict")
    );
    Ok(())
}

#[test]
fn projection_locator_runtime_check_rejects_relative_base_in_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let default_id = main.get_repo_info()?.expect("default repo").uuid;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 1,
            locators: vec![ProjectionLocatorRecord {
                repo_id: default_id,
                repo_name_hint: "default".into(),
                projection_base_abs: PathBuf::from("relative-notes"),
                canonicalized_at_unix_ms: 1,
            }],
        },
    )?;

    let err = main
        .check_projection_locator_for_local_repo("default")
        .expect_err("relative locator base must fail closed");
    assert!(err.to_string().contains("absolute projection base"));
    Ok(())
}

#[test]
fn projection_locator_list_validates_unknown_repo_records() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 1,
            locators: vec![ProjectionLocatorRecord {
                repo_id: uuid::Uuid::new_v4(),
                repo_name_hint: "ghost".into(),
                projection_base_abs: std::fs::canonicalize(&base)?,
                canonicalized_at_unix_ms: 1,
            }],
        },
    )?;

    let err = main
        .list_projection_locators()
        .expect_err("list must validate locator map before returning records");
    assert!(
        err.to_string()
            .contains("Projection Locator references unknown local repo")
    );
    Ok(())
}

#[test]
fn projection_locator_shared_base_allows_distinct_repo_roots() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    crate::test_support::create_initialized_local_repo(&ledger, "wiki", "urn:wiki");

    main.set_projection_base_for_local_repo("default", &base)?;
    main.set_projection_base_for_local_repo("wiki", &base)?;
    let default_id = main.get_repo_info()?.expect("default repo").uuid;
    let wiki_id = main
        .get_repo_info_for(None, Some("wiki"))?
        .expect("wiki repo")
        .uuid;

    assert_eq!(
        main.local_repo_workspace_root("default")?,
        std::fs::canonicalize(&base)?.join(repo_workspace_segment("default", default_id)?)
    );
    assert_eq!(
        main.local_repo_workspace_root("wiki")?,
        std::fs::canonicalize(&base)?.join(repo_workspace_segment("wiki", wiki_id)?)
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
    let mut main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    crate::test_support::create_initialized_local_repo(&ledger, "wiki", "urn:wiki");
    let default_id = main.get_repo_info()?.expect("default repo").uuid;
    let wiki_id = main
        .get_repo_info_for(None, Some("wiki"))?
        .expect("wiki repo")
        .uuid;

    main.set_projection_base_for_all_local_repos_checked(&old_base)?;
    main.set_projection_base_for_all_local_repos_checked(&new_base)?;

    let new_base = std::fs::canonicalize(&new_base)?;
    assert_eq!(
        main.local_repo_workspace_root("default")?,
        new_base.join(repo_workspace_segment("default", default_id)?)
    );
    assert_eq!(
        main.local_repo_workspace_root("wiki")?,
        new_base.join(repo_workspace_segment("wiki", wiki_id)?)
    );
    Ok(())
}

#[test]
fn projection_locator_nested_workspace_roots_fail_closed() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let main = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    crate::test_support::create_initialized_local_repo(&ledger, "wiki", "urn:wiki");
    let default_id = main.get_repo_info()?.expect("default repo").uuid;

    main.set_projection_base_for_local_repo("default", &base)?;
    let err = main
        .set_projection_base_for_local_repo(
            "wiki",
            base.join(repo_workspace_segment("default", default_id)?),
        )
        .expect_err("nested workspace roots must fail closed");

    assert!(
        err.to_string()
            .contains("Projection workspace nesting conflict")
    );
    Ok(())
}

#[test]
fn projection_locator_invalid_repo_name_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
    let repo_id = uuid::Uuid::new_v4();

    let err = repo
        .set_projection_base_for_repo_id(repo_id, "bad/name", &base)
        .expect_err("invalid repo path segment must fail closed");

    assert!(err.to_string().contains("single safe path segment"));
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
fn projection_locator_preserves_redb_suffix_in_repo_name_segment() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let repo = RepoManager::init(&ledger, 8, Some("paper.redb"), Some("urn:paper"))?;

    assert_eq!(repo.local_repo_name(), "paper.redb");

    repo.set_projection_base_for_local_repo("paper.redb", &base)?;

    assert_eq!(
        repo.local_repo_workspace_root("paper.redb")?,
        std::fs::canonicalize(&base)?.join(repo_workspace_segment(
            "paper.redb",
            repo.get_repo_info()?.expect("repo info").uuid
        )?)
    );
    assert_eq!(
        repo.projection_locator_for_local_repo("paper.redb")?
            .repo_name_hint,
        "paper.redb"
    );
    Ok(())
}

#[test]
fn projection_locator_normalized_workspace_key_detects_case_and_unicode_conflicts() {
    let base = PathBuf::from("notes");
    let id = uuid::Uuid::from_u128(1);

    assert_eq!(
        normalized_workspace_key(&base, &repo_workspace_segment("Default", id).unwrap()),
        normalized_workspace_key(&base, &repo_workspace_segment("default", id).unwrap())
    );
    assert_eq!(
        normalized_workspace_key(&base, &repo_workspace_segment("\u{00e9}", id).unwrap()),
        normalized_workspace_key(&base, &repo_workspace_segment("e\u{0301}", id).unwrap())
    );
    assert_ne!(
        normalized_workspace_key(&base, &repo_workspace_segment("default", id).unwrap()),
        normalized_workspace_key(
            &base,
            &repo_workspace_segment("default", uuid::Uuid::from_u128(2)).unwrap()
        )
    );
}

#[test]
fn projection_locator_rejects_protected_workspace_roots() -> anyhow::Result<()> {
    for protected_base in ["ledger/.host", ".git", ".notegit"] {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        let base = dir.path().join(protected_base);

        let err = repo
            .set_projection_base_for_local_repo("default", &base)
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
    let repo = Arc::new(RepoManager::init(
        &ledger,
        8,
        Some("default"),
        Some("urn:default"),
    )?);
    repo.set_projection_base_for_local_repo("default", &base)?;
    let workspace = repo.ensure_local_repo_workspace_identity("default")?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(workspace.join(".deveignore"), "ignored.md\n")?;
    std::fs::write(workspace.join("ignored.md"), "ignored")?;
    std::fs::write(workspace.join("kept.md"), "kept")?;

    SyncManager::new_checked(repo.clone())?.scan()?;

    let pending = repo.list_pending_fs_in_local_repo("default")?;
    let mut paths = pending
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, vec!["kept.md"]);
    Ok(())
}
