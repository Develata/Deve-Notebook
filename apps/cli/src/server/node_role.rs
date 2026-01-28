// apps/cli/src/server/node_role.rs
//! # Node Role State

use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug)]
pub struct NodeRole {
    pub role: String,
    pub ws_port: u16,
    pub main_port: u16,
}

static NODE_ROLE: OnceLock<Arc<NodeRole>> = OnceLock::new();

pub fn set_node_role(role: NodeRole) {
    let _ = NODE_ROLE.set(Arc::new(role));
}

pub fn get_node_role() -> Arc<NodeRole> {
    NODE_ROLE.get().cloned().unwrap_or_else(|| {
        Arc::new(NodeRole {
            role: "unknown".into(),
            ws_port: 0,
            main_port: 0,
        })
    })
}
