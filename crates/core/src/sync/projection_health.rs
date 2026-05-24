//! plan_ref:
//!   - 06_repository#tree-projection-contract
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
}

fn normalize(repo_name: &str) -> String {
    repo_name.to_string()
}
