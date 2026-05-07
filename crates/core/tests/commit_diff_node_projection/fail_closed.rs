use super::*;

#[test]
fn commit_diff_fails_closed_for_content_only_docs_without_structure_projection() {
    let (_dir, repo) = new_repo();
    let doc_id = deve_core::models::DocId::new();
    repo.append_local_op(&deve_core::models::LedgerEntry::new_content(
        doc_id,
        deve_core::models::Op::Insert {
            pos: 0,
            content: "orphan".into(),
        },
        0,
        deve_core::models::PeerId::new("test"),
        1,
        None,
        None,
    ))
    .expect("append orphan content op");
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "orphan-content", 1, ledger_seq)
        })
        .expect("create orphan commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("orphan content diff must fail closed");

    assert!(
        err.to_string()
            .contains("Commit diff lost projected path for doc")
    );
}

#[test]
fn commit_diff_fails_closed_on_missing_structure_targets() {
    let (_dir, repo) = new_repo();
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::MoveNode {
                node_id: NodeId::new(),
                doc_id: None,
                new_parent_id: None,
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "broken-structure", 0, ledger_seq)
        })
        .expect("create broken commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("broken structure diff must fail closed");
    assert!(err.to_string().contains("missing node"));
}

#[test]
fn commit_diff_fails_closed_when_doc_has_multiple_live_nodes() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::new(),
                doc_id,
                parent_id: None,
                name: "a.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::new(),
                doc_id,
                parent_id: None,
                name: "b.md".into(),
            },
            1,
            PeerId::new("test"),
            2,
        ),
    );
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "duplicate-doc", 0, ledger_seq)
        })
        .expect("create duplicate-doc commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("duplicate live doc paths must fail closed");

    assert!(
        err.to_string().contains("multiple live paths"),
        "unexpected error: {}",
        err
    );
}
