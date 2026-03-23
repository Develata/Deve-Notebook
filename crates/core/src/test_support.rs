use std::sync::{LazyLock, Mutex, MutexGuard};

static LOCAL_REPO_CATALOG_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn local_repo_catalog_test_guard() -> MutexGuard<'static, ()> {
    LOCAL_REPO_CATALOG_TEST_MUTEX
        .lock()
        .expect("local repo catalog test mutex")
}
