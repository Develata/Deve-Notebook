use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::conflict;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeDomain, ChangeStatus};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo
        .local_repo_workspace_path("default", path)
        .expect("workspace path");
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    let doc_id = repo
        .get_tracked_docid_in_local_repo(repo.local_repo_name(), path)
        .expect("resolve tracked doc id for pending seed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id,
                change_type: status,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending entry");
}

fn seed_initial_commit(repo: &RepoManager) -> deve_core::models::DocId {
    write_workspace_file(repo, "notes/a.md", "hello");
    seed_pending(repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage initial");
    repo.apply_external_changes()
        .expect("apply initial external change");
    repo.commit_source_control_changes("initial")
        .expect("initial commit");
    repo.get_tracked_docid_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect("doc id lookup")
        .expect("tracked doc id")
}

fn append_confirmed_ledger_edit(repo: &RepoManager, doc_id: deve_core::models::DocId) {
    let peer_id = PeerId::new("editor");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 5,
                    content: " world".into(),
                },
                1000,
                peer_id.clone(),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append editor ledger op");
}

fn append_confirmed_ledger_insert(
    repo: &RepoManager,
    doc_id: deve_core::models::DocId,
    content: &str,
) {
    let peer_id = PeerId::new("editor");
    let content = content.to_string();
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.clone().into(),
                },
                1000,
                peer_id.clone(),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append editor ledger insert");
}

fn ledger_head(repo: &RepoManager) -> u64 {
    repo.run_on_local_repo(
        repo.local_repo_name(),
        deve_core::ledger::range::get_max_seq,
    )
    .expect("ledger head")
}

#[test]
fn source_control_confirmed_added_doc_diff_contains_new_content() {
    let (_dir, repo) = new_repo();
    let base_head = ledger_head(&repo);
    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/new.md", None, "editor")
        .expect("create confirmed ledger doc");
    append_confirmed_ledger_insert(&repo, doc_id, "confirmed ledger smoke\nline 2");

    let confirmed = repo
        .list_confirmed_ledger_changes()
        .expect("confirmed ledger changes");
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].domain, ChangeDomain::ConfirmedLedger);
    assert_eq!(confirmed[0].doc_id, Some(doc_id));
    assert_eq!(confirmed[0].path, "notes/new.md");
    assert_eq!(confirmed[0].status, ChangeStatus::Added);
    assert_eq!(confirmed[0].base_seq, Some(base_head));
    assert_eq!(confirmed[0].target_seq, Some(ledger_head(&repo)));

    let diff = repo
        .diff_doc_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/new.md".into(),
                doc_id: Some(doc_id),
                domain: Some(ChangeDomain::ConfirmedLedger),
            },
        )
        .expect("confirmed added doc diff");

    assert!(diff.contains("+confirmed ledger smoke"), "{diff}");
    assert!(diff.contains("+line 2"), "{diff}");
}

#[test]
fn source_control_confirmed_ledger_dirty_tracks_editor_writes() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    let initial_head = ledger_head(&repo);

    append_confirmed_ledger_edit(&repo, doc_id);

    assert!(
        repo.list_pending_fs()
            .expect("pending after ledger edit")
            .is_empty(),
        "confirmed ledger dirty must not enter pending_fs_ops"
    );
    let confirmed = repo
        .list_confirmed_ledger_changes()
        .expect("confirmed ledger changes");
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].domain, ChangeDomain::ConfirmedLedger);
    assert_eq!(confirmed[0].doc_id, Some(doc_id));
    assert_eq!(confirmed[0].path, "notes/a.md");
    assert_eq!(confirmed[0].status, ChangeStatus::Modified);
    assert_eq!(confirmed[0].base_seq, Some(initial_head));
    assert_eq!(confirmed[0].target_seq, Some(ledger_head(&repo)));
}

#[test]
fn source_control_confirmed_only_commit_creates_anchor_without_new_ledger_facts() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    let dirty_head = ledger_head(&repo);

    let commit = repo
        .commit_source_control_changes("confirmed")
        .expect("confirmed-only commit");

    assert_eq!(commit.ledger_seq, dirty_head);
    assert_eq!(commit.doc_count, 1);
    assert_eq!(ledger_head(&repo), dirty_head);
    assert!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed after commit")
            .is_empty()
    );
}

#[test]
fn source_control_confirmed_only_commit_updates_committed_snapshot_base() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);

    repo.commit_source_control_changes("confirmed")
        .expect("confirmed-only commit");

    let pending_hash = pending_fs::content_hash("external edit");
    let has_conflict = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            conflict::check_conflict(db, doc_id, &pending_hash)
        })
        .expect("conflict check");

    assert!(
        !has_conflict,
        "confirmed-only commit must advance commit_snapshots with the commit anchor"
    );
}

#[test]
fn source_control_confirmed_only_commit_rejects_empty_message() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    let dirty_head = ledger_head(&repo);

    let err = repo
        .commit_source_control_changes("   ")
        .expect_err("empty confirmed-only commit message must fail");

    assert!(
        err.to_string().contains("non-empty message"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), dirty_head);
    assert_eq!(
        repo.list_commits_in_local_repo(repo.local_repo_name(), 10)
            .expect("commit history")
            .len(),
        1
    );
    assert_eq!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed remains dirty")
            .len(),
        1
    );
}

#[test]
fn source_control_diff_routes_working_and_confirmed_domains_independently() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");

    let changes = repo.list_changes().expect("flat changes");
    assert!(
        changes
            .iter()
            .any(|entry| entry.path == "notes/a.md"
                && entry.domain == ChangeDomain::WorkingDirectory)
    );
    assert!(
        changes.iter().any(
            |entry| entry.path == "notes/a.md" && entry.domain == ChangeDomain::ConfirmedLedger
        )
    );

    let working_diff = repo
        .diff_doc_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
                domain: Some(ChangeDomain::WorkingDirectory),
            },
        )
        .expect("working diff");
    let confirmed_diff = repo
        .diff_doc_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
                domain: Some(ChangeDomain::ConfirmedLedger),
            },
        )
        .expect("confirmed diff");

    assert!(working_diff.contains("+external"));
    assert!(confirmed_diff.contains("+hello world"));
}
