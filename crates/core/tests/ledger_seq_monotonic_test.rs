//! STORE-005: ledger_seq 跨操作单调递增。

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn global_seq_increases_across_content_ops() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "seq.md", None, "test")
        .expect("create file");
    let peer = PeerId::new("local");
    let mut prev_global = 0u64;
    for i in 0..5 {
        let (global, _local) = repo
            .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos: 0,
                        content: format!("op-{i}").into(),
                    },
                    i as i64,
                    peer.clone(),
                    seq,
                    None,
                    None,
                )
            })
            .expect("append op");
        assert!(
            global > prev_global,
            "global_seq must increase: {} > {}",
            global,
            prev_global
        );
        prev_global = global;
    }
}

#[test]
fn global_seq_increases_across_mixed_structure_and_content_ops() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = PeerId::new("local");

    // Structure op: create file (implicit global seq via structure facts)
    let (doc_a, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "a.md", None, "test")
        .expect("create a.md");

    // Content op on first doc
    let (g1, _) = repo
        .append_generated_op_in_local_repo(&name, doc_a, peer.clone(), |seq| {
            LedgerEntry::new_content(
                doc_a,
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
        .expect("content op a");

    // Structure op: create second file
    let (doc_b, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "b.md", None, "test")
        .expect("create b.md");

    // Content op on second doc
    let (g2, _) = repo
        .append_generated_op_in_local_repo(&name, doc_b, peer.clone(), |seq| {
            LedgerEntry::new_content(
                doc_b,
                Op::Insert {
                    pos: 0,
                    content: "world".into(),
                },
                2,
                peer.clone(),
                seq,
                None,
                None,
            )
        })
        .expect("content op b");

    assert!(
        g2 > g1,
        "global_seq must increase across interleaved ops: {} > {}",
        g2,
        g1
    );
}

#[test]
fn global_seq_increases_across_multiple_docs() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = PeerId::new("local");

    let mut docs = Vec::new();
    for i in 0..3 {
        let (doc_id, _ops) = repo
            .apply_file_structure_in_local_repo(&name, &format!("d{i}.md"), None, "test")
            .expect("create doc");
        docs.push(doc_id);
    }

    let mut prev_global = 0u64;
    for (i, &doc_id) in docs.iter().enumerate() {
        let (global, _) = repo
            .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos: 0,
                        content: format!("content-{i}").into(),
                    },
                    i as i64,
                    peer.clone(),
                    seq,
                    None,
                    None,
                )
            })
            .expect("append content op");
        assert!(global > prev_global, "monotonic across docs");
        prev_global = global;
    }
}
