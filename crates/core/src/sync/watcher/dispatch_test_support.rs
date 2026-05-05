use crate::ledger::RepoManager;
use crate::models::{LedgerEntry, PeerId};
use crate::sync::SyncManager;
use notify_debouncer_full::{
    DebouncedEvent,
    notify::{
        Event, EventKind,
        event::{ModifyKind, RenameMode},
    },
};
use redb::ReadableTable;
use std::sync::Arc;
use std::time::Instant;

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
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault)?;
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_vault_root_checked(&vault)?;
    let repo = Arc::new(repo);
    let sync = Arc::new(SyncManager::new_checked(repo.clone(), vault)?);
    let repo_name = repo.local_repo_name().to_string();
    let repo_id = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .expect("repo info")
        .uuid;
    let repo_root = repo.local_repo_workspace_root(&repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    Ok((dir, repo, sync, repo_name, repo_id, repo_root))
}

pub(super) fn event_for(path: std::path::PathBuf) -> DebouncedEvent {
    DebouncedEvent::new(
        Event {
            kind: EventKind::Any,
            paths: vec![path],
            attrs: Default::default(),
        },
        Instant::now(),
    )
}

pub(super) fn rename_event(old: std::path::PathBuf, new: std::path::PathBuf) -> DebouncedEvent {
    DebouncedEvent::new(
        Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![old, new],
            attrs: Default::default(),
        },
        Instant::now(),
    )
}

pub(super) fn commit_doc(
    repo: &Arc<RepoManager>,
    sync: &Arc<SyncManager>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<crate::models::DocId> {
    let (doc_id, _) = repo.apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    repo.append_generated_op_in_local_repo(repo_name, doc_id, PeerId::new("local"), |seq| {
        LedgerEntry::new_content(
            doc_id,
            crate::models::Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            PeerId::new("local"),
            seq,
            None,
            None,
        )
    })?;
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

pub(super) fn assert_fs_message(
    message: &crate::protocol::ServerMessage,
    path: &str,
    change_type: &str,
) {
    match message {
        crate::protocol::ServerMessage::FsChangeDetected {
            path: actual_path,
            change_type: actual_change_type,
            ..
        } => {
            assert_eq!(actual_path, path);
            assert_eq!(actual_change_type, change_type);
        }
        other => panic!("unexpected message: {other:?}"),
    }
}
