use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tracing::warn;

const DIR_REFRESH_COOLDOWN: Duration = Duration::from_millis(1500);

pub(crate) struct DirRefreshGuard {
    cooldown: Duration,
    recent: Mutex<HashMap<RepoId, Instant>>,
}

impl DirRefreshGuard {
    pub(crate) fn new() -> Self {
        Self {
            cooldown: DIR_REFRESH_COOLDOWN,
            recent: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    fn with_cooldown(cooldown: Duration) -> Self {
        Self {
            cooldown,
            recent: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn should_refresh(&self, repo_id: RepoId) -> bool {
        let Some(mut guard) = self.lock_recent() else {
            return false;
        };
        guard.retain(|_, at| at.elapsed() <= self.cooldown);
        match guard.get(&repo_id) {
            Some(at) if at.elapsed() <= self.cooldown => false,
            _ => {
                guard.insert(repo_id, Instant::now());
                true
            }
        }
    }

    fn lock_recent(&self) -> Option<MutexGuard<'_, HashMap<RepoId, Instant>>> {
        match self.recent.lock() {
            Ok(guard) => Some(guard),
            Err(_) => {
                warn!("DirRefreshGuard: recent 锁已损坏，按 fail-closed 处理");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DirRefreshGuard;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn throttles_same_repo_until_cooldown_expires() {
        let guard = DirRefreshGuard::with_cooldown(Duration::from_millis(10));
        let repo_id = Uuid::from_u128(1);
        assert!(guard.should_refresh(repo_id));
        assert!(!guard.should_refresh(repo_id));
        std::thread::sleep(Duration::from_millis(20));
        assert!(guard.should_refresh(repo_id));
    }

    #[test]
    fn poisoned_lock_blocks_refresh_fail_closed() {
        let guard = DirRefreshGuard::with_cooldown(Duration::from_millis(10));
        let repo_id = Uuid::from_u128(1);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _held = guard.recent.lock().expect("lock refresh guard");
            panic!("poison refresh guard");
        }));
        assert!(!guard.should_refresh(repo_id));
    }
}
