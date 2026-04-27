// apps/cli/src/server/node_role.rs
//! # Node Role State

use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug)]
pub struct NodeRole {
    pub role: String,
    pub ws_port: u16,
    pub main_port: u16,
    pub version: String,
    pub profile: String,
    pub delivery: String,
    pub environment: String,
}

static NODE_ROLE: OnceLock<Arc<NodeRole>> = OnceLock::new();

pub fn set_node_role(role: NodeRole) {
    if NODE_ROLE.set(Arc::new(role)).is_err() {
        tracing::warn!("NodeRole already set, ignoring duplicate call");
    }
}

pub fn get_node_role() -> Arc<NodeRole> {
    NODE_ROLE.get().cloned().unwrap_or_else(|| {
        Arc::new(NodeRole {
            role: "unknown".into(),
            ws_port: 0,
            main_port: 0,
            version: env!("CARGO_PKG_VERSION").into(),
            profile: "unknown".into(),
            delivery: "unknown".into(),
            environment: runtime_environment(),
        })
    })
}

pub fn runtime_environment() -> String {
    std::env::var("DEVE_ENV").unwrap_or_else(|_| "production".into())
}
