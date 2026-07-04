//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 18_release#runtime-observability
//!

mod epoch;
mod payload;
mod urls;

fn complete_node_role_payload(git_bridge: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
        "version": "0.0.1",
        "profile": "standard",
        "delivery": "embedded-frontend",
        "environment": "development",
        "source_control": {
            "git_bridge": git_bridge
        },
        "repo_health": {
            "status": "healthy",
            "local_total": 1,
            "healthy": 1,
            "degraded": 0
        },
        "host_file_actions": {
            "copy_absolute_path": true,
            "reveal_in_system_explorer": false
        }
    })
}
