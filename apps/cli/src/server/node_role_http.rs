// apps/cli/src/server/node_role_http.rs

use axum::response::IntoResponse;
use axum::Json;

use crate::server::node_role;

pub async fn role() -> impl IntoResponse {
    let r = node_role::get_node_role();
    Json(serde_json::json!({
        "role": r.role,
        "ws_port": r.ws_port,
        "main_port": r.main_port,
    }))
}
