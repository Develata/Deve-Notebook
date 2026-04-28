// apps/cli/src/server/node_role.rs
//! plan_ref:
//!   - 15_release#runtime-observability
//!
//! # Node Role State

use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Debug)]
pub struct NodeRole {
    pub role: String,
    pub ws_port: u16,
    pub main_port: u16,
    pub version: String,
    pub profile: String,
    pub delivery: String,
    pub environment: String,
    pub repo_health: RepoHealthSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoHealthSummary {
    pub status: String,
    pub local_total: usize,
    pub healthy: usize,
    pub degraded: usize,
}

impl RepoHealthSummary {
    pub fn unknown() -> Self {
        Self {
            status: "unknown".into(),
            local_total: 0,
            healthy: 0,
            degraded: 0,
        }
    }

    pub fn from_degraded_count(local_total: usize, degraded: usize) -> Self {
        let healthy = local_total.saturating_sub(degraded);
        let status = if degraded > 0 { "degraded" } else { "healthy" };
        Self {
            status: status.into(),
            local_total,
            healthy,
            degraded,
        }
    }
}

static NODE_ROLE: OnceLock<Arc<RwLock<NodeRole>>> = OnceLock::new();

pub fn set_node_role(role: NodeRole) {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => *current = role,
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring role update"),
    }
}

pub fn update_repo_health(repo_health: RepoHealthSummary) {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => current.repo_health = repo_health,
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring repo health update"),
    }
}

pub fn get_node_role() -> NodeRole {
    let cell = role_cell();
    match cell.read() {
        Ok(current) => current.clone(),
        Err(_) => {
            tracing::warn!("NodeRole lock poisoned, returning unknown role");
            default_node_role()
        }
    }
}

pub fn runtime_environment() -> String {
    std::env::var("DEVE_ENV").unwrap_or_else(|_| "production".into())
}

fn role_cell() -> Arc<RwLock<NodeRole>> {
    NODE_ROLE
        .get_or_init(|| Arc::new(RwLock::new(default_node_role())))
        .clone()
}

fn default_node_role() -> NodeRole {
    NodeRole {
        role: "unknown".into(),
        ws_port: 0,
        main_port: 0,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: "unknown".into(),
        delivery: "unknown".into(),
        environment: runtime_environment(),
        repo_health: RepoHealthSummary::unknown(),
    }
}
