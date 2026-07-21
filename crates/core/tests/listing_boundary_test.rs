//! Listing boundary tests — empty repo, deleted doc, multi-repo isolation.

use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{LedgerEntry, Op, RepoType};
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, repo)
}

#[test]
fn empty_repo_listing_returns_empty_vec() {
    let (_dir, repo) = new_repo();
    let repo_id = repo.get_repo_info().unwrap().unwrap().uuid;

    let docs = repo
        .list_docs(&RepoType::Local(repo_id))
        .expect("list docs on empty repo");
    assert!(docs.is_empty(), "empty repo must return empty listing");

    let nodes = repo
        .list_nodes(&RepoType::Local(repo_id))
        .expect("list nodes on empty repo");
    assert!(
        nodes.is_empty(),
        "empty repo must return empty node listing"
    );
}

#[test]
fn deleted_doc_does_not_appear_in_listing() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let repo_id = repo.get_repo_info().unwrap().unwrap().uuid;

    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(&name, "notes/temp.md", None, "test")
        .expect("create file");
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("write content");

    let before = repo
        .list_docs(&RepoType::Local(repo_id))
        .expect("list before delete");
    assert_eq!(before.len(), 1);

    repo.apply_file_delete_structure_in_local_repo(&name, "notes/temp.md", None, "test")
        .expect("delete file");

    let after = repo
        .list_docs(&RepoType::Local(repo_id))
        .expect("list after delete");
    assert!(
        !after.iter().any(|(id, _)| *id == doc_id),
        "deleted doc must not appear in listing"
    );
}

#[test]
fn multi_repo_listing_isolation() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger = dir.path().join("ledger");
    let (alpha, alpha_id) =
        common::init_cataloged_repo_with_depth(&ledger, &dir.path().join("alpha-notes"), 10)
            .expect("init alpha");
    let beta_id = common::add_cataloged_repo_with_depth(&alpha, &dir.path().join("beta-notes"), 10)
        .expect("init beta");

    let (doc_a, _) = alpha
        .apply_file_structure_in_local_repo(alpha.local_repo_name(), "readme.md", None, "test")
        .expect("create in alpha");
    let (doc_b, _) = alpha
        .apply_file_structure_in_local_repo(&beta_id.to_string(), "readme.md", None, "test")
        .expect("create in beta");

    let alpha_docs = alpha
        .list_docs(&RepoType::Local(alpha_id))
        .expect("list alpha");
    let beta_docs = alpha
        .list_docs(&RepoType::Local(beta_id))
        .expect("list beta");

    assert_eq!(alpha_docs.len(), 1);
    assert_eq!(alpha_docs[0].0, doc_a);
    assert_eq!(beta_docs.len(), 1);
    assert_eq!(beta_docs[0].0, doc_b);
    assert_ne!(
        doc_a, doc_b,
        "same path in different repos must have different doc IDs"
    );
}
