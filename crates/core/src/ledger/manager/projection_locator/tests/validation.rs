use super::super::*;
use crate::sync::SyncManager;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn projection_locator_record_query_rejects_duplicate_repo_id() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;
    let repo_id = uuid::Uuid::new_v4();

    write_projection_locator_file(
        &projection_locator_path_for(&ledger),
        &ProjectionLocatorFile {
            version: 2,
            locators: vec![
                ProjectionLocatorRecord {
                    repo_id,
                    workspace_segment: "default".into(),
                    projection_base_abs: std::fs::canonicalize(&base)?,
                    canonicalized_at_unix_ms: 1,
                },
                ProjectionLocatorRecord {
                    repo_id,
                    workspace_segment: "duplicate".into(),
                    projection_base_abs: std::fs::canonicalize(&base)?,
                    canonicalized_at_unix_ms: 2,
                },
            ],
        },
    )?;

    let err = projection_locator_record_for_repo_id(&ledger, repo_id)
        .expect_err("duplicate locator records must fail closed");
    assert!(err.to_string().contains("duplicate record"));
    Ok(())
}

#[test]
fn projection_locator_runtime_check_validates_full_map_conflicts() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;
    let (main, default_id) = super::init_cataloged_repo(&ledger, &base)?;
    let wiki_base = dir.path().join("wiki-notes");
    let (_, wiki_id) = super::init_cataloged_repo(&ledger, &wiki_base)?;
    let nested_base = base.join(default_id.to_string());
    std::fs::create_dir_all(&nested_base)?;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 2,
            locators: vec![
                ProjectionLocatorRecord {
                    repo_id: default_id,
                    workspace_segment: default_id.to_string(),
                    projection_base_abs: std::fs::canonicalize(&base)?,
                    canonicalized_at_unix_ms: 1,
                },
                ProjectionLocatorRecord {
                    repo_id: wiki_id,
                    workspace_segment: wiki_id.to_string(),
                    projection_base_abs: std::fs::canonicalize(&nested_base)?,
                    canonicalized_at_unix_ms: 1,
                },
            ],
        },
    )?;

    let err = main
        .check_projection_locator_for_local_repo(&default_id.to_string())
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
    let base = dir.path().join("notes");
    let (main, default_id) = super::init_cataloged_repo(&ledger, &base)?;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 2,
            locators: vec![ProjectionLocatorRecord {
                repo_id: default_id,
                workspace_segment: default_id.to_string(),
                projection_base_abs: PathBuf::from("relative-notes"),
                canonicalized_at_unix_ms: 1,
            }],
        },
    )?;

    let err = main
        .check_projection_locator_for_local_repo(&default_id.to_string())
        .expect_err("relative locator base must fail closed");
    assert!(err.to_string().contains("absolute projection base"));
    Ok(())
}

#[test]
fn projection_locator_list_validates_unknown_repo_records() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (main, _) = super::init_cataloged_repo(&ledger, &base)?;

    write_projection_locator_file(
        &main.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 2,
            locators: vec![ProjectionLocatorRecord {
                repo_id: uuid::Uuid::new_v4(),
                workspace_segment: uuid::Uuid::new_v4().to_string(),
                projection_base_abs: std::fs::canonicalize(&base)?,
                canonicalized_at_unix_ms: 1,
            }],
        },
    )?;

    assert!(
        main.list_projection_locators()?.is_empty(),
        "unknown locator truth must stay hidden from the normal catalog view"
    );
    Ok(())
}

#[test]
fn prepared_locator_does_not_block_cataloged_repo_queries() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let (main, healthy_id) = super::init_cataloged_repo(&ledger, &base)?;
    let prepared_id = uuid::Uuid::new_v4();
    let prepared = RepoManager::init_with_options(
        &ledger,
        8,
        Some(&prepared_id.to_string()),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(prepared_id),
            repo_url: None,
        },
    )?;
    prepared.prepare_projection_locator_for_repo_creation(prepared_id, &base)?;

    let visible = main.list_projection_locators()?;
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].repo_id, healthy_id);
    assert_eq!(
        main.check_projection_locator_for_local_repo(&healthy_id.to_string())?,
        visible[0]
            .projection_base_abs
            .join(&visible[0].workspace_segment)
    );
    assert!(
        main.projection_locator_for_local_repo(&prepared_id.to_string())
            .is_err(),
        "prepared raw truth must not enter normal workspace admission"
    );
    Ok(())
}

#[test]
fn prepared_locator_is_raw_recovery_truth_until_catalog_cut() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    let repo_id = uuid::Uuid::new_v4();
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        &ledger,
        8,
        Some(&execution_name),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;

    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, &base)?;
    assert_eq!(locator.workspace_segment, execution_name);
    assert!(
        repo.list_projection_locators()?.is_empty(),
        "normal list must hide prepared locator without blocking healthy repos"
    );
    assert_eq!(
        repo.query_projection_locator_record_for_repo_id(repo_id)?,
        Some(locator.clone())
    );

    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    crate::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &repo_id.to_string())?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;

    assert_eq!(repo.list_projection_locators()?, vec![locator]);
    Ok(())
}

#[test]
fn prepared_locator_rejects_conflicting_immutable_segment() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;
    let repo_id = uuid::Uuid::new_v4();
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        &ledger,
        8,
        Some(&execution_name),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;
    write_projection_locator_file(
        &repo.projection_locator_path(),
        &ProjectionLocatorFile {
            version: 2,
            locators: vec![ProjectionLocatorRecord {
                repo_id,
                workspace_segment: "foreign-segment".into(),
                projection_base_abs: std::fs::canonicalize(&base)?,
                canonicalized_at_unix_ms: 1,
            }],
        },
    )?;

    let error = repo
        .prepare_projection_locator_for_repo_creation(repo_id, &base)
        .expect_err("prepared retry cannot mutate immutable segment");
    assert!(
        error
            .to_string()
            .contains("conflicting immutable workspace segment")
    );
    Ok(())
}

#[test]
fn concurrent_locator_map_writes_do_not_drop_entries() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let base = dir.path().join("notes");
    std::fs::create_dir_all(&base)?;
    let ids: Vec<uuid::Uuid> = (0..4).map(|_| uuid::Uuid::new_v4()).collect();
    let managers = ids
        .iter()
        .map(|id| {
            RepoManager::init_with_options(
                &ledger,
                4,
                Some(&id.to_string()),
                crate::ledger::init::RepoInitOptions {
                    repo_id: Some(*id),
                    repo_url: None,
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let handles: Vec<_> = managers
        .into_iter()
        .zip(ids.iter().copied())
        .map(|(repo, id)| {
            let base = base.clone();
            std::thread::spawn(move || {
                repo.prepare_projection_locator_for_repo_creation(id, &base)
                    .map(|_| ())
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("prepare thread panicked")?;
    }

    let file = read_projection_locator_file(&projection_locator_path_for(&ledger))?;
    assert_eq!(
        file.locators.len(),
        ids.len(),
        "every concurrently prepared locator entry must survive the map rewrite"
    );
    Ok(())
}
