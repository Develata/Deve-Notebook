//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::WatcherError;
use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::thread::JoinHandle;

pub(crate) struct WatcherHandle {
    pub stop_tx: std::sync::mpsc::Sender<()>,
    pub join: JoinHandle<Result<(), WatcherError>>,
}

enum WatcherSlot {
    Running(WatcherHandle),
    Stopping,
}

static REGISTRY: LazyLock<Mutex<HashMap<RepoId, WatcherSlot>>> =
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
    guard.insert(repo_id, WatcherSlot::Running(handle));
    Ok(())
}

pub(crate) fn is_running(repo_id: RepoId) -> Result<bool, WatcherError> {
    let guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    Ok(guard.contains_key(&repo_id))
}

pub(crate) fn begin_stop(repo_id: RepoId) -> Result<Option<WatcherHandle>, WatcherError> {
    let mut guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    match guard.remove(&repo_id) {
        Some(WatcherSlot::Running(handle)) => {
            guard.insert(repo_id, WatcherSlot::Stopping);
            Ok(Some(handle))
        }
        Some(WatcherSlot::Stopping) => {
            guard.insert(repo_id, WatcherSlot::Stopping);
            Ok(None)
        }
        None => Ok(None),
    }
}

pub(crate) fn finish_stop(repo_id: RepoId) -> Result<(), WatcherError> {
    let mut guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    if matches!(guard.get(&repo_id), Some(WatcherSlot::Stopping)) {
        guard.remove(&repo_id);
    }
    Ok(())
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
        let _ = begin_stop(repo_id);
        let _ = finish_stop(repo_id);

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
        let original = begin_stop(repo_id)
            .expect("begin stop original watcher")
            .expect("original watcher still registered");
        release_first.send(()).expect("release original thread");
        original.stop_tx.send(()).expect("stop original");
        original
            .join
            .join()
            .expect("original watcher thread join")
            .expect("original watcher thread result");
        finish_stop(repo_id).expect("finish original stop");
        assert!(!is_running(repo_id).expect("registry readable"));
    }

    #[test]
    fn stopping_slot_blocks_new_watcher_until_finish_stop() {
        let repo_id = uuid::Uuid::from_u128(18);
        let _ = begin_stop(repo_id);
        let _ = finish_stop(repo_id);

        let (first, release_first, _first_done) = parked_handle();
        match insert_or_reject(repo_id, first) {
            Ok(()) => {}
            Err((err, _)) => panic!("insert first watcher: {err}"),
        }
        let original = begin_stop(repo_id)
            .expect("begin stop original watcher")
            .expect("original watcher still registered");
        assert!(is_running(repo_id).expect("stopping slot blocks start"));

        let (second, release_second, _second_done) = parked_handle();
        let rejected = insert_or_reject(repo_id, second)
            .expect_err("stopping repo must reject new watcher")
            .0;
        assert!(matches!(rejected, WatcherError::AlreadyRunning(id) if id == repo_id));
        release_second.send(()).expect("release rejected thread");

        release_first.send(()).expect("release original thread");
        original.stop_tx.send(()).expect("stop original");
        original
            .join
            .join()
            .expect("original watcher thread join")
            .expect("original watcher thread result");
        finish_stop(repo_id).expect("finish original stop");
        assert!(!is_running(repo_id).expect("registry readable"));
    }
}
