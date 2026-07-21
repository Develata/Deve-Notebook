use deve_core::ledger::listing::RepoListing;
use deve_core::models::DocId;
use deve_core::source_control::staging;
use tempfile::TempDir;

mod common;

#[test]
fn local_catalog_fails_closed_on_missing_secondary_source_control_tables_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("main-notes"), 8)
            .expect("main");
    let legacy_id =
        common::add_cataloged_repo_with_depth(&repo, &dir.path().join("legacy-notes"), 8)
            .expect("legacy");
    // Drop the secondary repo's source-control tables to simulate a legacy repo
    // created before those tables existed. The repo keeps its Normal catalog
    // membership, so it stays visible to resolution but fails closed on any
    // source-control access until an explicit repair.
    let legacy_db = repo
        .lease_local_authority(legacy_id)
        .expect("legacy authority lease");
    let write = legacy_db.db().begin_write().expect("write txn");
    let _ = write
        .delete_table(staging::STAGED_TABLE)
        .expect("delete staged table");
    write.commit().expect("commit table delete");
    let legacy_selector = legacy_id.to_string();

    let list_err = repo
        .list_repos(None)
        .expect_err("missing source control tables must fail local listing");
    assert!(list_err.to_string().contains("source control tables"));
    let pending_err = repo
        .list_pending_fs_in_local_repo(&legacy_selector)
        .expect_err("missing source control tables must fail local pending listing");
    let pending_detail = pending_err.to_string();
    assert!(
        pending_detail.contains("source control tables"),
        "unexpected pending error: {pending_detail}"
    );

    repo.repair_local_repo_catalog()
        .expect("repair local repo catalog");
    let mut expected = vec![legacy_id.to_string(), main_id.to_string()];
    expected.sort();
    assert_eq!(
        repo.list_repos(None)
            .expect("list local repos after repair"),
        expected
    );
    assert!(
        repo.list_pending_fs_in_local_repo(&legacy_selector)
            .expect("list pending after repair")
            .is_empty()
    );
}

#[test]
fn main_local_repo_fails_closed_on_missing_source_control_tables_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");

    let handle = repo
        .lease_local_authority(main_id)
        .expect("main authority lease");
    let write = handle.db().begin_write().expect("write txn");
    let _ = write
        .delete_table(staging::STAGED_TABLE)
        .expect("delete staged table");
    write.commit().expect("commit table delete");

    let staged_err = repo
        .list_staged()
        .expect_err("missing source control tables must fail staged listing");
    assert!(
        staged_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(staged_err.to_string().contains("source control tables"));

    let commits_err = repo
        .list_commits(10)
        .expect_err("missing source control tables must fail commit listing");
    assert!(
        commits_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(commits_err.to_string().contains("source control tables"));

    let content_err = repo
        .get_committed_content(DocId::new())
        .expect_err("missing source control tables must fail committed content lookup");
    assert!(
        content_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(content_err.to_string().contains("source control tables"));

    repo.repair_local_repo_catalog()
        .expect("repair local repo catalog");
    assert!(
        repo.list_staged()
            .expect("list staged after repair")
            .is_empty()
    );
    assert!(
        repo.list_commits(10)
            .expect("list commits after repair")
            .is_empty()
    );
}
