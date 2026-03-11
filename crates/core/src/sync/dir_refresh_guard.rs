use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
        let mut guard = self.recent.lock().unwrap();
        guard.retain(|_, at| at.elapsed() <= self.cooldown);
        match guard.get(&repo_id) {
            Some(at) if at.elapsed() <= self.cooldown => false,
            _ => {
                guard.insert(repo_id, Instant::now());
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DirRefreshGuard;
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
}
