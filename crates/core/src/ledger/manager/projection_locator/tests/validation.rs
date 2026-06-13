use super::super::*;
use crate::sync::SyncManager;
use std::path::PathBuf;
use std::sync::Arc;

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
