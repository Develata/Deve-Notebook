//! plan_ref:
//!   - 04_storage#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::sync::watcher::{self, WatcherError};
use watcher_test_support::Harness;

#[test]
fn watcher_duplicate_start_fails_and_can_restart_after_stop() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let first = watcher::start_repo_watcher(h.sync.clone(), &repo_name, None, None)?;

    let duplicate = watcher::start_repo_watcher(h.sync.clone(), &repo_name, None, None)
        .expect_err("same repo must not start a second watcher");
    assert!(matches!(duplicate, WatcherError::AlreadyRunning(id) if id == first));

    watcher::stop_repo_watcher(first)?;
    let restarted = watcher::start_repo_watcher(h.sync.clone(), &repo_name, None, None)?;
    watcher::stop_repo_watcher(restarted)?;
    Ok(())
}
