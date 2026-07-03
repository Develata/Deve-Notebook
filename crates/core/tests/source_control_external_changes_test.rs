use deve_core::config::GitBridgeMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeDomain, ChangeStatus, SourceControlApi, staging};
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

fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path("default", path)
        .expect("workspace path")
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
    repo.commit_staged_with_git_bridge("initial", GitBridgeMode::Off)
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

fn ledger_head(repo: &RepoManager) -> u64 {
    repo.run_on_local_repo(
        repo.local_repo_name(),
        deve_core::ledger::range::get_max_seq,
    )
    .expect("ledger head")
}

#[test]
fn external_changes_apply_writes_ledger_without_commit_anchor() {
    let (_dir, repo) = new_repo();
    let before_head = ledger_head(&repo);
    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");

    let confirmed = repo
        .apply_external_changes()
        .expect("apply external changes");

    assert!(ledger_head(&repo) > before_head);
    assert!(repo.list_staged().expect("staged after apply").is_empty());
    assert!(
        repo.list_pending_fs()
            .expect("pending after apply")
            .is_empty()
    );
    assert!(
        repo.list_commits(10)
            .expect("commits after external apply")
            .is_empty(),
        "Apply to Ledger must not create a Source Control commit anchor"
    );
    assert!(confirmed.iter().any(|entry| {
        entry.path == "notes/external.md"
            && entry.domain == ChangeDomain::ConfirmedLedger
            && entry.status == ChangeStatus::Added
    }));
}

#[test]
fn source_control_commit_ignores_external_staged_when_confirmed_is_empty() {
    let (_dir, repo) = new_repo();
    let before_head = ledger_head(&repo);
    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");

    let err =
        <RepoManager as SourceControlApi>::commit_source_control_changes_in_repo_with_git_bridge(
            &repo,
            &RepoSelector::default(),
            "version anchor",
            GitBridgeMode::Off,
        )
        .expect_err("Source Control commit must not consume external staged changes");

    assert!(
        err.to_string()
            .contains("confirmed ledger changes are empty"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_head);
    assert_eq!(repo.list_staged().expect("staged retained").len(), 1);
    assert!(
        repo.list_commits(10)
            .expect("commits after rejected commit")
            .is_empty()
    );
}

#[test]
fn source_control_commit_preserves_external_staging_while_committing_confirmed() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");
    append_confirmed_ledger_edit(&repo, doc_id);
    let dirty_head = ledger_head(&repo);

    let commit =
        <RepoManager as SourceControlApi>::commit_source_control_changes_in_repo_with_git_bridge(
            &repo,
            &RepoSelector::default(),
            "version anchor",
            GitBridgeMode::Off,
        )
        .expect("confirmed ledger commit should ignore external staging");

    assert_eq!(commit.ledger_seq, dirty_head);
    assert_eq!(commit.doc_count, 1);
    assert_eq!(ledger_head(&repo), dirty_head);
    assert_eq!(
        repo.list_staged().expect("external staged retained").len(),
        1
    );
    assert!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed after commit")
            .is_empty()
    );
}

#[test]
fn external_stage_rejects_overlap_with_confirmed_ledger_dirty() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");

    let err = repo
        .stage_pending("notes/a.md")
        .expect_err("overlapping external change must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "read surface must flag external/confirmed overlap for UI"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_stage_rejects_docless_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless rename pending");

    let err = repo
        .stage_pending("notes/renamed.md")
        .expect_err("docless rename overlap must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "docless rename overlap must be surfaced as conflict state"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_stage_fails_closed_on_same_path_different_doc_id_overlap() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/a.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(DocId::new()),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed mismatched doc pending");

    let err = repo
        .stage_pending("notes/a.md")
        .expect_err("same-path different-doc overlap must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes")
            || err.to_string().contains("Path is not in pending_fs_ops"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "same-path doc mismatch must surface as conflict state"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_apply_rejects_staged_overlap_with_confirmed_ledger_dirty() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");
    repo.stage_pending("notes/a.md")
        .expect("stage external before ledger dirty");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("overlapping staged external change must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "staged overlap must be surfaced as conflict state"
    );
    assert_eq!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed retained")
            .len(),
        1
    );
}

#[test]
fn external_apply_rejects_docless_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless rename staged");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("docless rename overlap must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "docless staged rename overlap must be surfaced as conflict state"
    );
}

#[test]
fn external_apply_rejects_docful_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(DocId::new()),
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docful mismatched rename staged");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("docful rename overlap must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "docful staged rename overlap must be surfaced as conflict state"
    );
}

#[test]
fn external_discard_allows_staged_overlap_without_touching_confirmed_ledger() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");
    repo.stage_pending("notes/a.md")
        .expect("stage external before ledger dirty");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_discard_head = ledger_head(&repo);

    repo.discard_pending("notes/a.md")
        .expect("discard staged external overlap");

    let restored = std::fs::read_to_string(workspace_path(&repo, "notes/a.md"))
        .expect("read restored projection");
    assert_eq!(restored, "hello world");
    assert_eq!(ledger_head(&repo), before_discard_head);
    assert!(repo.list_staged().expect("staged cleared").is_empty());
    assert!(
        repo.run_on_local_repo(repo.local_repo_name(), staging::list_staged_entries)
            .expect("raw staged")
            .is_empty()
    );
    assert_eq!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed still dirty")
            .len(),
        1
    );
}
