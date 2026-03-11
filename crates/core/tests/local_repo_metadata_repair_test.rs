use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use tempfile::TempDir;

fn write_info(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit");
}

#[test]
fn init_repairs_duplicate_local_repo_uuid_and_name_drift() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;

    let bad = RepoInfo {
        uuid: main_info.uuid,
        name: "main".into(),
        url: Some(format!("urn:uuid:{}", main_info.uuid)),
    };
    write_info(wiki_db.as_ref(), &bad);
    main.repair_local_repo_catalog().expect("repair catalog");

    let repaired_main = main
        .get_repo_info()
        .expect("repaired main info")
        .expect("main present");
    let repaired_wiki = main
        .get_repo_info_for(None, Some("wiki"))
        .expect("wiki lookup")
        .expect("wiki present");

    assert_eq!(repaired_main.name, "main");
    assert_eq!(repaired_wiki.name, "wiki");
    assert_ne!(repaired_wiki.uuid, repaired_main.uuid);
    assert_eq!(
        repaired_wiki.url.as_deref(),
        Some(format!("urn:uuid:{}", repaired_wiki.uuid).as_str())
    );
}
