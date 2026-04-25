//! Scope switch invariant tests — multi-repo doc isolation and shadow write barriers.

use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, RepoType, StructureOp};
use tempfile::TempDir;
use uuid::Uuid;

mod common;

fn two_local_repos() -> (TempDir, RepoManager, Uuid, Uuid) {
    let dir = TempDir::new().expect("create tempdir");
    let ledger = dir.path().join("ledger");
    let main =
        RepoManager::init(&ledger, 10, Some("main"), Some("urn:main")).expect("init main repo");
    let wiki_info =
        common::create_initialized_local_repo_with_depth(&ledger, 10, "wiki", "urn:wiki");
    let main_id = main.get_repo_info().unwrap().unwrap().uuid;
    let wiki_id = wiki_info.uuid;
    (dir, main, main_id, wiki_id)
}

#[test]
fn docs_in_one_local_repo_are_invisible_to_another() {
    let (_dir, repo, main_id, wiki_id) = two_local_repos();

    let (doc_a, _) = repo
        .apply_file_structure_in_local_repo("main", "notes/a.md", None, "test")
        .expect("create in main");
    let (doc_b, _) = repo
        .apply_file_structure_in_local_repo("wiki", "notes/b.md", None, "test")
        .expect("create in wiki");

    let main_docs = repo
        .list_docs(&RepoType::Local(main_id))
        .expect("list main");
    let wiki_docs = repo
        .list_docs(&RepoType::Local(wiki_id))
        .expect("list wiki");

    assert!(main_docs.iter().any(|(id, _)| *id == doc_a));
    assert!(
        !main_docs.iter().any(|(id, _)| *id == doc_b),
        "wiki doc must not appear in main listing"
    );

    assert!(wiki_docs.iter().any(|(id, _)| *id == doc_b));
    assert!(
        !wiki_docs.iter().any(|(id, _)| *id == doc_a),
        "main doc must not appear in wiki listing"
    );
}

#[test]
fn tracked_docid_is_isolated_across_local_repos() {
    let (_dir, repo, _, _) = two_local_repos();

    let (doc_a, _) = repo
        .apply_file_structure_in_local_repo("main", "notes/shared-name.md", None, "test")
        .expect("create in main");

    let wiki_result = repo
        .get_tracked_docid_in_local_repo("wiki", "notes/shared-name.md")
        .expect("query wiki");
    assert_eq!(
        wiki_result, None,
        "path tracked in main must not resolve in wiki"
    );

    let main_result = repo
        .get_tracked_docid_in_local_repo("main", "notes/shared-name.md")
        .expect("query main");
    assert_eq!(main_result, Some(doc_a));
}

#[test]
fn shadow_doc_does_not_leak_into_any_local_repo() {
    let (_dir, repo, main_id, wiki_id) = two_local_repos();

    let peer = PeerId::new("remote-peer");
    let remote_repo_id = Uuid::new_v4();
    let remote_doc = DocId::new();
    repo.append_remote_op(
        &peer,
        &remote_repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(remote_doc),
                doc_id: remote_doc,
                parent_id: None,
                name: "shadow.md".into(),
            },
            1,
            peer.clone(),
            1,
        ),
    )
    .expect("append remote structure op");

    let main_docs = repo
        .list_docs(&RepoType::Local(main_id))
        .expect("list main");
    let wiki_docs = repo
        .list_docs(&RepoType::Local(wiki_id))
        .expect("list wiki");

    assert!(
        !main_docs.iter().any(|(id, _)| *id == remote_doc),
        "shadow doc must not appear in main"
    );
    assert!(
        !wiki_docs.iter().any(|(id, _)| *id == remote_doc),
        "shadow doc must not appear in wiki"
    );

    let shadow_docs = repo
        .list_docs(&RepoType::Remote(peer, remote_repo_id))
        .expect("list shadow");
    assert_eq!(shadow_docs.len(), 1);
    assert_eq!(shadow_docs[0].0, remote_doc);
}
