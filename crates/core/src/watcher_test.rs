use super::Watcher;
use crate::ledger::{REPO_METADATA, RepoManager};
use crate::sync::SyncManager;
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

fn new_repo() -> (tempfile::TempDir, Arc<RepoManager>, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(&vault);
    (dir, Arc::new(repo), vault)
}

#[test]
fn watcher_fails_closed_on_dir_change_resolution_error() {
    let (_dir, repo, vault) = new_repo();
    let docs = vault.join("default/docs");
    std::fs::create_dir_all(&docs).expect("mkdir docs");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write
            .open_table(REPO_METADATA)?
            .insert(&0, b"not-bincode".as_slice())?;
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("poison repo metadata");

    let sync = Arc::new(SyncManager::new(repo, vault.clone()));
    let watcher = Watcher::new(sync, vault.clone());
    let err = watcher
        .process_events(
            &[DebouncedEvent::new(docs, DebouncedEventKind::Any)],
            &vault,
        )
        .expect_err("dir change resolution must fail closed");

    assert!(
        err.to_string()
            .contains("Failed to handle dir change for default/docs")
    );
}

#[cfg(unix)]
#[test]
fn watcher_fails_closed_on_unstatable_dir_event() {
    let (_dir, repo, vault) = new_repo();
    let blocked = vault.join("default/blocked");
    std::fs::create_dir_all(&blocked).expect("mkdir blocked");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");

    let sync = Arc::new(SyncManager::new(repo, vault.clone()));
    let watcher = Watcher::new(sync, vault.clone());
    let err = watcher
        .process_events(
            &[DebouncedEvent::new(
                blocked.clone(),
                DebouncedEventKind::Any,
            )],
            &vault,
        )
        .expect_err("unstatable dir event must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to classify watcher event for default/blocked")
            || err
                .to_string()
                .contains("Failed to handle dir change for default/blocked")
            || err.to_string().contains("Permission denied")
    );
}
