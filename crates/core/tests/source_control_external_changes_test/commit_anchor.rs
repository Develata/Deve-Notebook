use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::source_control::{ChangeStatus, SourceControlApi};

use super::support::{
    append_confirmed_ledger_edit, ledger_head, new_repo, seed_initial_commit, seed_pending,
    write_workspace_file,
};

#[test]
fn source_control_commit_ignores_external_staged_when_confirmed_is_empty() {
    let (_dir, repo) = new_repo();
    let before_head = ledger_head(&repo);
    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");

    let err = <RepoManager as SourceControlApi>::commit_source_control_changes_in_repo(
        &repo,
        &RepoSelector::default(),
        "version anchor",
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

    let commit = <RepoManager as SourceControlApi>::commit_source_control_changes_in_repo(
        &repo,
        &RepoSelector::default(),
        "version anchor",
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
