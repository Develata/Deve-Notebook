//! plan_ref:
//!   - 04_storage#watcher-contract

use super::WatcherError;
use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::thread::JoinHandle;

pub(crate) struct WatcherHandle {
    pub stop_tx: std::sync::mpsc::Sender<()>,
    pub join: JoinHandle<Result<(), WatcherError>>,
}

static REGISTRY: LazyLock<Mutex<HashMap<RepoId, WatcherHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn insert_or_reject(
    repo_id: RepoId,
    handle: WatcherHandle,
) -> Result<(), (WatcherError, WatcherHandle)> {
    let mut guard = match REGISTRY.lock() {
        Ok(guard) => guard,
        Err(_) => return Err((WatcherError::RegistryPoisoned, handle)),
    };
    if guard.contains_key(&repo_id) {
        return Err((WatcherError::AlreadyRunning(repo_id), handle));
    }
    guard.insert(repo_id, handle);
    Ok(())
}

pub(crate) fn is_running(repo_id: RepoId) -> Result<bool, WatcherError> {
    let guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    Ok(guard.contains_key(&repo_id))
}

pub(crate) fn remove(repo_id: RepoId) -> Result<Option<WatcherHandle>, WatcherError> {
    let mut guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    Ok(guard.remove(&repo_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn parked_handle() -> (
        WatcherHandle,
        mpsc::Sender<()>,
        mpsc::Receiver<Result<(), WatcherError>>,
    ) {
        let (stop_tx, stop_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let _ = release_rx.recv();
            let _ = done_tx.send(Ok(()));
            stop_rx
                .recv()
                .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
            Ok(())
        });
        (WatcherHandle { stop_tx, join }, release_tx, done_rx)
    }

    #[test]
    fn duplicate_insert_rejects_second_handle_without_replacing_existing() {
        let repo_id = uuid::Uuid::from_u128(17);
        let _ = remove(repo_id);

        let (first, release_first, _first_done) = parked_handle();
        match insert_or_reject(repo_id, first) {
            Ok(()) => {}
            Err((err, _)) => panic!("insert first watcher: {err}"),
        }
        assert!(is_running(repo_id).expect("registry readable"));

        let (second, release_second, _second_done) = parked_handle();
        let rejected = insert_or_reject(repo_id, second)
            .expect_err("duplicate watcher handle must be rejected")
            .0;
        assert!(matches!(rejected, WatcherError::AlreadyRunning(id) if id == repo_id));

        release_second.send(()).expect("release rejected thread");
        let original = remove(repo_id)
            .expect("remove original watcher")
            .expect("original watcher still registered");
        release_first.send(()).expect("release original thread");
        original.stop_tx.send(()).expect("stop original");
        original
            .join
            .join()
            .expect("original watcher thread join")
            .expect("original watcher thread result");
        assert!(!is_running(repo_id).expect("registry readable"));
    }
}
