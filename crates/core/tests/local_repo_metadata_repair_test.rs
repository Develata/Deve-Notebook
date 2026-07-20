use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

mod common;

#[test]
fn repair_fails_closed_on_repo_id_mismatch_without_rewriting_identity() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("main-notes"), 8)
            .expect("main");
    let (_wiki, wiki_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("wiki-notes"), 8)
            .expect("wiki");
    let wiki_db = main
        .open_database(None, &wiki_id.to_string())
        .expect("wiki db")
        .db;

    let bad = RepoInfo {
        uuid: main_id,
        name: main_id.to_string(),
        url: Some(format!("urn:uuid:{}", main_id)),
    };
    common::write_repo_metadata(wiki_db.as_ref(), &bad);
    let err = main
        .repair_local_repo_catalog()
        .expect_err("repair must not rewrite RepoId identity");
    assert!(err.to_string().contains("physical RepoId does not match"));
    assert_eq!(common::read_repo_metadata(wiki_db.as_ref()), bad);
}

#[test]
fn local_repo_duplicate_display_aliases_are_valid_and_never_resolve_selectors() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) = common::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("main-notes"),
        "urn:test:wiki-a",
    )
    .expect("main");
    let (_second, second_id) = common::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("second-notes"),
        "urn:test:wiki-b",
    )
    .expect("second");
    main.host_repo_alias_runtime()
        .set_alias(main_id, "wiki", 0)
        .expect("alias main");
    main.host_repo_alias_runtime()
        .set_alias(second_id, "wiki", 0)
        .expect("duplicate display aliases are valid");

    let repos = main
        .list_repos(None)
        .expect("duplicate display aliases keep both repos listed");
    assert_eq!(repos.len(), 2);
    assert!(repos.contains(&main_id.to_string()));
    assert!(repos.contains(&second_id.to_string()));
    // Display aliases are host-local UI state and are never consulted by
    // resolution — a shared alias is legal precisely because it can never make
    // a machine selector ambiguous. Resolution by the alias string fails
    // closed as not-found; canonical machine names are RepoId strings.
    let err = main
        .resolve_local_repo_name_for_execution(None, Some("wiki"))
        .expect_err("display alias must not resolve as machine selector");
    assert!(
        err.to_string()
            .contains("Local repo not found for name wiki")
    );
}

#[test]
fn local_repo_execution_requires_explicit_selector_when_multiple_repos_exist() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("main-notes"), 8)
            .expect("main");
    let (_wiki, _wiki_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("wiki-notes"), 8)
            .expect("wiki");

    let err = main
        .resolve_local_repo_name_for_execution(None, None)
        .expect_err("multiple repos must require explicit selector");

    assert!(err.to_string().contains("multiple local repos exist"));
}

#[test]
fn repair_fails_closed_on_duplicate_local_repo_url_without_rewriting_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) = common::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("main-notes"),
        "urn:main",
    )
    .expect("main");
    let (_notes, notes_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes-notes"), 8)
            .expect("notes");
    let other_db = main
        .open_database(None, &notes_id.to_string())
        .expect("notes db")
        .db;
    common::write_repo_metadata(
        other_db.as_ref(),
        &RepoInfo {
            uuid: notes_id,
            name: notes_id.to_string(),
            url: Some("urn:main".into()),
        },
    );
    let err = main
        .repair_local_repo_catalog()
        .expect_err("repair must not rewrite duplicate URL identity");
    assert!(err.to_string().contains("duplicate local repository URL"));
    assert_eq!(
        common::read_repo_metadata(other_db.as_ref()).url.as_deref(),
        Some("urn:main")
    );
}

#[test]
fn init_without_url_does_not_reuse_same_name_repo_with_explicit_url() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let first = RepoManager::init(
        &ledger_dir,
        8,
        Some("wiki"),
        Some("https://example.com/wiki.git"),
    )
    .expect("init explicit wiki");
    let second = RepoManager::init(&ledger_dir, 8, Some("wiki"), None).expect("init implicit wiki");

    let first_info = first.get_repo_info().expect("first info").expect("present");
    let second_info = second
        .get_repo_info()
        .expect("second info")
        .expect("present");
    assert_eq!(first_info.name, "wiki");
    assert_eq!(second_info.name, "wiki");
    assert_ne!(first_info.uuid, second_info.uuid);
}

#[test]
fn init_keeps_duplicate_display_name_for_same_name_different_url() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let first = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("https://a.example"))
        .expect("init first wiki");
    let second = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("https://b.example"))
        .expect("init second wiki");

    let first_info = first.get_repo_info().expect("first info").expect("present");
    let second_info = second
        .get_repo_info()
        .expect("second info")
        .expect("present");
    assert_eq!(first_info.name, "wiki");
    assert_eq!(second_info.name, "wiki");
    assert_eq!(first_info.url.as_deref(), Some("https://a.example"));
    assert_eq!(second_info.url.as_deref(), Some("https://b.example"));
    assert_ne!(first_info.uuid, second_info.uuid);
    assert!(
        ledger_dir
            .join("local")
            .join(format!("{}.redb", first_info.uuid))
            .exists()
    );
    assert!(
        ledger_dir
            .join("local")
            .join(format!("{}.redb", second_info.uuid))
            .exists()
    );
}

#[test]
fn init_fails_closed_on_existing_local_repo_without_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo_id = uuid::Uuid::new_v4();
    common::seed_metadata_less_local_repo(&ledger_dir, &repo_id.to_string());

    let err = match RepoManager::init(&ledger_dir, 8, Some("legacy"), None) {
        Ok(_) => panic!("missing repo metadata must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("repository metadata missing"));
}

#[test]
fn local_execution_resolution_ignores_broken_remote_catalogs() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, _repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    let peer_id = deve_core::models::PeerId::new("peer-a");
    common::seed_broken_remote_shadow_repo(&ledger_dir, &peer_id, "broken");

    assert_eq!(
        repo.resolve_local_repo_name_for_execution(None, Some(repo.local_repo_name()))
            .expect("local execution selector"),
        repo.local_repo_name()
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("local repo names"),
        vec![repo.local_repo_name().to_string()]
    );
}
