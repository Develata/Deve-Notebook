//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! In-process repo-scoped writer gate for local browser edit commits.

use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static REPO_WRITE_GATES: LazyLock<Mutex<HashMap<RepoId, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn with_repo_write_gate<T>(
    repo_id: RepoId,
    body: impl FnOnce() -> T,
) -> Result<T, ServerError> {
    let gate = {
        let mut gates = REPO_WRITE_GATES.lock().map_err(|_| poisoned_error())?;
        gates
            .entry(repo_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };

    let _guard = gate.lock().map_err(|_| poisoned_error())?;
    Ok(body())
}

fn poisoned_error() -> ServerError {
    ServerError::with_detail(
        ServerErrorCode::StoragePersistFailed,
        "repo writer gate is poisoned",
    )
}

#[cfg(test)]
mod tests {
    use super::with_repo_write_gate;
    use deve_core::models::RepoId;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn repo_write_gate_serializes_same_repo() {
        let repo_id = RepoId::new_v4();
        let (entered_first_tx, entered_first_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();
        let (entered_second_tx, entered_second_rx) = mpsc::channel();

        let first = std::thread::spawn(move || {
            with_repo_write_gate(repo_id, || {
                entered_first_tx.send(()).unwrap();
                release_first_rx.recv().unwrap();
            })
            .unwrap();
        });

        entered_first_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();

        let second = std::thread::spawn(move || {
            with_repo_write_gate(repo_id, || {
                entered_second_tx.send(()).unwrap();
            })
            .unwrap();
        });

        assert!(
            entered_second_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err()
        );

        release_first_tx.send(()).unwrap();
        entered_second_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();

        first.join().unwrap();
        second.join().unwrap();
    }
}
