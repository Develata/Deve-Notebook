//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

pub(super) use super::super::source_control_test_support::ProxyHarness;
use deve_core::ledger::RepoManager;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::TempDir;

pub(super) fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    seed_pending_entry(
        repo,
        PendingFsEntry {
            path: path.into(),
            renamed_from: None,
            doc_id: None,
            change_type: status,
            content_hash: pending_fs::content_hash(content),
            detected_at: 1,
            has_conflict: false,
        },
    );
}

pub(super) fn seed_tracked_rename(
    repo: &RepoManager,
    doc_id: DocId,
    old_path: &str,
    new_path: &str,
    content: &str,
) {
    seed_pending_entry(repo, rename_deleted_entry(doc_id, old_path));
    seed_pending_entry(
        repo,
        PendingFsEntry {
            path: new_path.into(),
            renamed_from: Some(old_path.into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash(content),
            detected_at: 1,
            has_conflict: false,
        },
    );
}

pub(super) fn path_target(path: &str) -> ScPathTarget {
    ScPathTarget::from_path(path)
}

pub(super) fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("notes").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending_entry(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

fn rename_deleted_entry(doc_id: DocId, path: &str) -> PendingFsEntry {
    PendingFsEntry {
        path: path.into(),
        renamed_from: None,
        doc_id: Some(doc_id),
        change_type: ChangeStatus::Deleted,
        content_hash: String::new(),
        detected_at: 1,
        has_conflict: false,
    }
}
