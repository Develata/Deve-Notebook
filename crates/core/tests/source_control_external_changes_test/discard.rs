use deve_core::source_control::{ChangeStatus, staging};

use super::support::{
    append_confirmed_ledger_edit, ledger_head, new_repo, seed_initial_commit, seed_pending,
    workspace_path, write_workspace_file,
};

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
