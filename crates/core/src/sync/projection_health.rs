//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!
//! Runtime health gate for repos with recoverable projection faults.

use std::collections::HashSet;
use std::sync::RwLock;

pub(super) struct ProjectionHealth {
    degraded_repos: RwLock<HashSet<String>>,
}

impl ProjectionHealth {
    pub(super) fn new() -> Self {
        Self {
            degraded_repos: RwLock::new(HashSet::new()),
        }
    }

    pub(super) fn replace_degraded(&self, repo_names: &[String]) {
        match self.degraded_repos.write() {
            Ok(mut repos) => {
                *repos = repo_names.iter().map(|name| normalize(name)).collect();
            }
            Err(err) => tracing::error!("Failed to update degraded repo set: {}", err),
        }
    }

    pub(super) fn mark_degraded(&self, repo_name: &str) {
        match self.degraded_repos.write() {
            Ok(mut repos) => {
                repos.insert(normalize(repo_name));
            }
            Err(err) => tracing::error!("Failed to mark degraded repo: {}", err),
        }
    }

    pub(super) fn clear_degraded(&self, repo_name: &str) {
        match self.degraded_repos.write() {
            Ok(mut repos) => {
                repos.remove(normalize(repo_name).as_str());
            }
            Err(err) => tracing::error!("Failed to clear degraded repo: {}", err),
        }
    }

    pub(super) fn is_degraded(&self, repo_name: &str) -> bool {
        self.degraded_repos
            .read()
            .map(|repos| repos.contains(normalize(repo_name).as_str()))
            .unwrap_or(true)
    }

    pub(super) fn degraded_snapshot(&self) -> Result<HashSet<String>, String> {
        self.degraded_repos
            .read()
            .map(|repos| repos.clone())
            .map_err(|error| error.to_string())
    }
}

fn normalize(repo_name: &str) -> String {
    repo_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::ProjectionHealth;

    #[test]
    fn degraded_snapshot_is_single_lock_and_reports_poison() {
        let health = ProjectionHealth::new();
        health.mark_degraded("repo-a");
        assert_eq!(
            health.degraded_snapshot().expect("health snapshot"),
            std::collections::HashSet::from(["repo-a".to_string()])
        );

        let lock = &health.degraded_repos;
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let _guard = lock.write().expect("projection health test lock");
                panic!("poison projection health test lock");
            });
            assert!(handle.join().is_err());
        });
        assert!(health.degraded_snapshot().is_err());
    }
}
