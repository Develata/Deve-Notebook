//! STORE-005: ledger_seq 跨操作单调递增。

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op};
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
    let peer = repo.local_peer_id().clone();
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
    let peer = repo.local_peer_id().clone();

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
    let peer = repo.local_peer_id().clone();

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

#[test]
fn peer_fact_seq_is_contiguous_across_structure_and_content_facts() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = repo.local_peer_id().clone();
    repo.apply_dir_create_structure_in_local_repo(&name, "notes", "test")
        .expect("create dir");
    let (doc_a, _) = repo
        .apply_file_structure_in_local_repo(&name, "notes/a.md", None, "test")
        .expect("create a");
    repo.local_fact_writer(deve_core::models::FactActor::new("test").unwrap())
        .append_content_in_local_repo(
            &name,
            doc_a,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
            1,
        )
        .expect("append a");
    let (doc_b, _) = repo
        .apply_file_structure_in_local_repo(&name, "notes/b.md", None, "test")
        .expect("create b");
    repo.local_fact_writer(deve_core::models::FactActor::new("test").unwrap())
        .append_content_in_local_repo(
            &name,
            doc_b,
            Op::Insert {
                pos: 0,
                content: "b".into(),
            },
            2,
        )
        .expect("append b");

    let repo_id = repo.get_repo_info().unwrap().unwrap().uuid;
    let waterline = repo.get_local_peer_waterline(&repo_id).unwrap();
    let entries = repo
        .get_local_ops_in_range(&repo_id, &peer, 1_u64.into(), waterline)
        .unwrap();
    assert_eq!(entries.len() as u64, waterline.get());
    for (expected, (_global_seq, entry)) in (1_u64..).zip(entries) {
        assert_eq!(entry.origin_peer_id, peer);
        assert_eq!(entry.peer_seq, expected);
    }
}

#[test]
fn failed_local_append_does_not_consume_peer_fact_seq() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer = repo.local_peer_id().clone();
    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(&name, "rollback.md", None, "test")
        .expect("create doc");
    let before = repo
        .get_local_peer_waterline(&repo.get_repo_info().unwrap().unwrap().uuid)
        .unwrap();

    let error = repo
        .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "bad".into(),
                },
                1,
                peer.clone(),
                seq + 1,
                None,
                None,
            )
        })
        .expect_err("mismatched peer sequence must roll back");
    assert!(
        error.to_string().contains("sequence mismatch"),
        "unexpected error: {error:#}"
    );

    let (_global, peer_seq) = repo
        .local_fact_writer(deve_core::models::FactActor::new("test").unwrap())
        .append_content_in_local_repo(
            &name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: "good".into(),
            },
            2,
        )
        .expect("append after rollback");
    assert_eq!(peer_seq, before.get() + 1);
}
