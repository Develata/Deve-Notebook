use std::sync::{LazyLock, Mutex, MutexGuard};

use crate::ledger::{RepoInfo, RepoManager};

static LOCAL_REPO_CATALOG_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn local_repo_catalog_test_guard() -> MutexGuard<'static, ()> {
    LOCAL_REPO_CATALOG_TEST_MUTEX
        .lock()
        .expect("local repo catalog test mutex")
}

pub(crate) fn create_initialized_local_repo(
    ledger_dir: &std::path::Path,
    name: &str,
    url: &str,
) -> RepoInfo {
    create_initialized_local_repo_with_depth(ledger_dir, 8, name, url)
}

pub(crate) fn create_initialized_local_repo_with_depth(
    ledger_dir: &std::path::Path,
    snapshot_depth: usize,
    name: &str,
    url: &str,
) -> RepoInfo {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, Some(name), Some(url))
        .expect("initialized local repo");
    repo.get_repo_info()
        .expect("local repo info")
        .expect("local repo metadata")
}
