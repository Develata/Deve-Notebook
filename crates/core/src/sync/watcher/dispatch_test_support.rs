use crate::ledger::RepoManager;
use crate::models::{FactActor, Op};
use crate::sync::SyncManager;
use redb::ReadableTable;
use std::sync::Arc;

pub(super) type SyncFixture = (
    tempfile::TempDir,
    Arc<RepoManager>,
    Arc<SyncManager>,
    String,
    crate::models::RepoId,
    std::path::PathBuf,
);

pub(super) fn new_sync() -> anyhow::Result<SyncFixture> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    std::fs::create_dir_all(&projection_base)?;
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let sync = Arc::new(SyncManager::new_checked(repo.clone())?);
    let repo_name = repo.local_repo_name().to_string();
    let repo_id = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .expect("repo info")
        .uuid;
    let repo_root = repo.local_repo_workspace_root(&repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    Ok((dir, repo, sync, repo_name, repo_id, repo_root))
}

pub(super) fn event_for(
    repo_root: &std::path::Path,
    path: std::path::PathBuf,
) -> super::backend::FsEventHint {
    super::backend::FsEventHint::changed(event_path(repo_root, &path))
}

pub(super) fn rename_event(
    repo_root: &std::path::Path,
    old: std::path::PathBuf,
    new: std::path::PathBuf,
) -> super::backend::FsEventHint {
    super::backend::FsEventHint::rename(event_path(repo_root, &old), event_path(repo_root, &new))
}

pub(super) fn removed_dir_event(
    repo_root: &std::path::Path,
    path: std::path::PathBuf,
) -> super::backend::FsEventHint {
    super::backend::FsEventHint::removed_directory(event_path(repo_root, &path))
}

fn event_path(repo_root: &std::path::Path, path: &std::path::Path) -> super::backend::FsEventPath {
    let relative = path
        .strip_prefix(repo_root)
        .expect("test event must remain inside the repo root");
    super::backend::FsEventPath::new(crate::utils::path::path_to_forward_slash(relative))
        .expect("valid repo-relative event path")
}

pub(super) fn commit_doc(
    repo: &Arc<RepoManager>,
    sync: &Arc<SyncManager>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<crate::models::DocId> {
    let (doc_id, _) = repo.apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    repo.local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            repo_name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )?;
    sync.persist_doc_in_local_repo(repo_name, doc_id)?;
    Ok(doc_id)
}

pub(super) fn ledger_op_count(repo: &Arc<RepoManager>, repo_name: &str) -> anyhow::Result<usize> {
    repo.run_on_local_repo(repo_name, |db| {
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(crate::ledger::schema::LEDGER_OPS)?;
        let mut count = 0;
        for item in table.iter()? {
            item?;
            count += 1;
        }
        Ok(count)
    })
}

pub(super) fn assert_fs_message(message: &super::WatcherRefresh, path: &str, change_type: &str) {
    assert_eq!(message.path(), path);
    let expected = match change_type {
        "added" => super::WatcherRefreshKind::Added,
        "modified" => super::WatcherRefreshKind::Modified,
        "deleted" => super::WatcherRefreshKind::Deleted,
        "dir_changed" => super::WatcherRefreshKind::DirectoryChanged,
        other => panic!("unexpected refresh kind fixture: {other}"),
    };
    assert_eq!(message.kind(), expected);
}
