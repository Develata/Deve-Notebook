use deve_core::ledger::RepoManager;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, staging};
use tempfile::tempdir;

fn new_repo() -> RepoManager {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    repo
}

fn pending_entry(path: &str, doc_id: DocId) -> PendingFsEntry {
    PendingFsEntry {
        path: path.into(),
        renamed_from: None,
        doc_id: Some(doc_id),
        change_type: ChangeStatus::Modified,
        content_hash: pending_fs::content_hash("body"),
        detected_at: 1,
        has_conflict: false,
    }
}

#[test]
fn pending_target_with_doc_id_does_not_fall_back_to_path_match() {
    let repo = new_repo();
    let wanted_doc = DocId::new();
    let other_doc = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(db, &pending_entry("notes/reused.md", other_doc))?;
        let target = ScPathTarget {
            path: "notes/reused.md".into(),
            doc_id: Some(wanted_doc),
        };
        let found = pending_fs::get_for_target(db, &target)?;
        assert!(found.is_none());
        Ok(())
    })
    .expect("query pending target");
}

#[test]
fn pending_target_with_doc_id_still_resolves_rename_successor() {
    let repo = new_repo();
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let mut entry = pending_entry("notes/new.md", doc_id);
        entry.renamed_from = Some("notes/old.md".into());
        pending_fs::upsert(db, &entry)?;
        let target = ScPathTarget {
            path: "notes/old.md".into(),
            doc_id: Some(doc_id),
        };
        let found = pending_fs::get_for_target(db, &target)?;
        assert_eq!(found.expect("rename successor").path, "notes/new.md");
        Ok(())
    })
    .expect("query pending rename");
}

#[test]
fn staged_target_with_doc_id_does_not_fall_back_to_path_match() {
    let repo = new_repo();
    let wanted_doc = DocId::new();
    let other_doc = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(db, &pending_entry("notes/reused.md", other_doc))?;
        let target = ScPathTarget {
            path: "notes/reused.md".into(),
            doc_id: Some(wanted_doc),
        };
        let found = staging::get_staged_for_target(db, &target)?;
        assert!(found.is_none());
        Ok(())
    })
    .expect("query staged target");
}

#[test]
fn staged_target_with_doc_id_still_resolves_rename_successor() {
    let repo = new_repo();
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let mut entry = pending_entry("notes/new.md", doc_id);
        entry.renamed_from = Some("notes/old.md".into());
        staging::stage_pending_entry(db, &entry)?;
        let target = ScPathTarget {
            path: "notes/old.md".into(),
            doc_id: Some(doc_id),
        };
        let found = staging::get_staged_for_target(db, &target)?;
        assert_eq!(found.expect("rename successor").0, "notes/new.md");
        Ok(())
    })
    .expect("query staged rename");
}
