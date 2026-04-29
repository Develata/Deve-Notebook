//! plan_ref:
//!   - 15_release#runtime-observability
//!

use gloo_net::http::Request;
use leptos::prelude::*;

pub(super) async fn fetch_node_role(ws_url: String, set_node_role: WriteSignal<String>) {
    fetch_node_role_for_http_base(http_base_from_ws_url(&ws_url), set_node_role).await;
}

pub(super) async fn fetch_node_role_for_http_base(
    http_base: String,
    set_node_role: WriteSignal<String>,
) {
    let http_url = http_base.trim_end_matches('/');
    let url = format!("{}/api/node/role", http_url);
    let res = Request::get(&url).send().await;
    if let Ok(resp) = res
        && let Ok(json) = resp.json::<serde_json::Value>().await
    {
        set_node_role.set(format_node_role_summary(&json));
    }
}

pub(super) fn http_base_from_ws_url(ws_url: &str) -> String {
    ws_url
        .replace("wss://", "https://")
        .replace("ws://", "http://")
        .trim_end_matches("/ws")
        .to_string()
}

fn format_node_role_summary(json: &serde_json::Value) -> String {
    let role = str_field(json, "role", "unknown");
    let main_port = json.get("main_port").and_then(|v| v.as_u64()).unwrap_or(0);
    let ws_port = json.get("ws_port").and_then(|v| v.as_u64()).unwrap_or(0);
    let version = str_field(json, "version", "unknown-version");
    let profile = str_field(json, "profile", "unknown-profile");
    let delivery = str_field(json, "delivery", "unknown-delivery");
    let environment = str_field(json, "environment", "unknown-env");
    let repo_health = format_repo_health(json);

    let role_text = if role == "proxy" && main_port > 0 {
        format!("proxy -> {} (ws:{})", main_port, ws_port)
    } else if ws_port > 0 {
        format!("{} (ws:{})", role, ws_port)
    } else {
        role.to_string()
    };
    format!(
        "{} | v{} | {} | {} | {} | repos:{}",
        role_text, version, profile, delivery, environment, repo_health
    )
}

fn str_field<'a>(json: &'a serde_json::Value, key: &str, fallback: &'a str) -> &'a str {
    json.get(key).and_then(|v| v.as_str()).unwrap_or(fallback)
}

fn format_repo_health(json: &serde_json::Value) -> String {
    let Some(repo_health) = json.get("repo_health") else {
        return "unknown".into();
    };
    let status = str_field(repo_health, "status", "unknown");
    let total = repo_health
        .get("local_total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let degraded = repo_health
        .get("degraded")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    format!("{} ({}/{})", status, degraded, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formats_main_runtime_summary() {
        let summary = format_node_role_summary(&json!({
            "role": "main",
            "ws_port": 3001,
            "main_port": 3001,
            "version": "0.0.1",
            "profile": "standard",
            "delivery": "embedded-frontend",
            "environment": "development",
            "repo_health": {
                "status": "healthy",
                "local_total": 1,
                "healthy": 1,
                "degraded": 0
            }
        }));

        assert_eq!(
            summary,
            "main (ws:3001) | v0.0.1 | standard | embedded-frontend | development | repos:healthy (0/1)"
        );
    }

    #[test]
    fn formats_proxy_runtime_summary() {
        let summary = format_node_role_summary(&json!({
            "role": "proxy",
            "ws_port": 3002,
            "main_port": 3001,
            "version": "0.0.1",
            "profile": "proxy",
            "delivery": "plugin-host-proxy",
            "environment": "production"
        }));

        assert_eq!(
            summary,
            "proxy -> 3001 (ws:3002) | v0.0.1 | proxy | plugin-host-proxy | production | repos:unknown"
        );
    }

    #[test]
    fn formats_degraded_repo_health() {
        let summary = format_node_role_summary(&json!({
            "role": "main",
            "ws_port": 3001,
            "main_port": 3001,
            "version": "0.0.1",
            "profile": "standard",
            "delivery": "embedded-frontend",
            "environment": "development",
            "repo_health": {
                "status": "degraded",
                "local_total": 2,
                "healthy": 1,
                "degraded": 1
            }
        }));

        assert!(summary.contains("repos:degraded (1/2)"));
    }

    #[test]
    fn derives_http_base_from_ws_url() {
        assert_eq!(
            http_base_from_ws_url("ws://127.0.0.1:3001/ws"),
            "http://127.0.0.1:3001"
        );
        assert_eq!(
            http_base_from_ws_url("wss://example.test/ws"),
            "https://example.test"
        );
    }
}
