use super::run;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use tempfile::TempDir;

fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("structure");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append op");
    doc_id
}

#[test]
fn markdown_export_supports_single_doc_output() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("init repo");
    let doc_id = seed_doc(&repo, "notes/a.md", "hello export");
    let output = dir.path().join("single.md");

    run(
        &ledger_dir,
        Some(output.display().to_string()),
        None,
        Some(doc_id.to_string()),
        8,
        "markdown",
    )
    .expect("export markdown doc");

    assert_eq!(
        std::fs::read_to_string(output).expect("read export"),
        "hello export"
    );
}

#[test]
fn json_export_rejects_single_doc_selector() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let err = run(
        &ledger_dir,
        None,
        None,
        Some(uuid::Uuid::new_v4().to_string()),
        8,
        "json",
    )
    .expect_err("json export should reject --doc");

    assert!(
        err.to_string()
            .contains("JSON export does not support --doc"),
        "unexpected error: {err:#}"
    );
}
