//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::sync::watcher::{self, WatcherError};
use std::time::Duration;
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

#[test]
fn watcher_rejects_zero_debounce_window() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();

    let err = watcher::start_repo_watcher(h.sync.clone(), &repo_name, Some(Duration::ZERO), None)
        .expect_err("zero debounce must fail closed before watcher start");

    assert!(matches!(err, WatcherError::ZeroDebounce));
    Ok(())
}
