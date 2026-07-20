//! STORE-005: Ledger-first round-trip tests.

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op};
use deve_core::state;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("tempdir");
    let (repo, _repo_id) =
        common::init_cataloged_repo(&dir.path().join("ledger"), &dir.path().join("notes"))
            .expect("init cataloged repo");
    (dir, repo)
}

/// Insert round-trip: ops reconstruct to the expected content.
#[test]
fn edit_round_trip_reconstructs_content() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = repo.local_peer_id().clone();

    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(&name, "rt.md", None, "test")
        .expect("create file");

    // Insert "hello" at pos 0
    let (g1, _) = repo
        .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
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
        .expect("insert hello");

    // Insert " world" at pos 5
    let (g2, _) = repo
        .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 5,
                    content: " world".into(),
                },
                2,
                peer.clone(),
                seq,
                None,
                None,
            )
        })
        .expect("insert world");

    assert!(g2 > g1, "seq must be monotonic");

    let ops = repo
        .get_local_ops_in_local_repo(&name, doc_id)
        .expect("get ops");
    let entries: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
    let reconstructed = state::reconstruct_content(&entries);
    assert_eq!(reconstructed, "hello world");
}

/// Delete round-trip: delete op correctly removes characters.
#[test]
fn delete_op_round_trip() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = repo.local_peer_id().clone();

    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(&name, "del.md", None, "test")
        .expect("create file");

    // Insert "abcdef"
    repo.append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "abcdef".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("insert");

    // Delete first 3 chars (UTF-16 code units)
    repo.append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Delete { pos: 0, len: 3 },
            2,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("delete");

    let ops = repo
        .get_local_ops_in_local_repo(&name, doc_id)
        .expect("get ops");
    let entries: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
    let reconstructed = state::reconstruct_content(&entries);
    assert_eq!(reconstructed, "def");
}
