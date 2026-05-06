//! plan_ref:
//!   - 15_release#runtime-observability
//!

use futures::FutureExt;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;

const NODE_ROLE_PROBE_RETRIES: usize = 3;
const NODE_ROLE_PROBE_RETRY_DELAY_MS: u32 = 150;
const NODE_ROLE_PROBE_TIMEOUT_MS: u32 = 1_500;

pub(super) async fn fetch_node_role(
    ws_url: String,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) {
    fetch_node_role_for_http_base(
        http_base_from_ws_url(&ws_url),
        set_node_role,
        set_node_role_probe_failed,
        current_connection_epoch,
        probe_connection_epoch,
    )
    .await;
}

pub(super) async fn fetch_node_role_for_http_base(
    http_base: String,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) {
    let http_url = http_base.trim_end_matches('/');
    let url = format!("{}/api/node/role", http_url);

    for attempt in 0..NODE_ROLE_PROBE_RETRIES {
        if let Some(json) = fetch_node_role_json_with_timeout(&url).await {
            apply_node_role_probe_success(
                set_node_role,
                set_node_role_probe_failed,
                current_connection_epoch,
                probe_connection_epoch,
                format_node_role_summary(&json),
            );
            return;
        }

        if attempt + 1 < NODE_ROLE_PROBE_RETRIES {
            TimeoutFuture::new(NODE_ROLE_PROBE_RETRY_DELAY_MS).await;
        }
    }

    leptos::logging::error!("Node role probe failed after retries: {}", url);
    apply_node_role_probe_failure(
        set_node_role,
        set_node_role_probe_failed,
        current_connection_epoch,
        probe_connection_epoch,
    );
}

async fn fetch_node_role_json_with_timeout(url: &str) -> Option<serde_json::Value> {
    let request = async {
        let resp = Request::get(url).send().await.ok()?;
        if !resp.ok() {
            return None;
        }
        resp.json::<serde_json::Value>().await.ok()
    }
    .fuse();
    let timeout = TimeoutFuture::new(NODE_ROLE_PROBE_TIMEOUT_MS).fuse();
    futures::pin_mut!(request, timeout);

    futures::select! {
        result = request => result,
        _ = timeout => None,
    }
}

fn apply_node_role_probe_success(
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
    summary: String,
) -> bool {
    if current_connection_epoch.get_untracked() != probe_connection_epoch {
        return false;
    }
    set_node_role.set(summary);
    set_node_role_probe_failed.set(false);
    true
}

fn apply_node_role_probe_failure(
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) -> bool {
    if current_connection_epoch.get_untracked() != probe_connection_epoch {
        return false;
    }
    set_node_role.set(String::new());
    set_node_role_probe_failed.set(true);
    true
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

    #[test]
    fn stale_node_role_probe_results_do_not_mutate_current_connection() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (node_role, set_node_role) = signal("main".to_string());
        let (probe_failed, set_probe_failed) = signal(false);
        let (connection_epoch, set_connection_epoch) = signal(2u64);

        assert!(!apply_node_role_probe_failure(
            set_node_role,
            set_probe_failed,
            connection_epoch,
            1,
        ));
        assert_eq!(node_role.get_untracked(), "main");
        assert!(!probe_failed.get_untracked());

        assert!(apply_node_role_probe_failure(
            set_node_role,
            set_probe_failed,
            connection_epoch,
            2,
        ));
        assert_eq!(node_role.get_untracked(), "");
        assert!(probe_failed.get_untracked());

        set_connection_epoch.set(3);
        assert!(!apply_node_role_probe_success(
            set_node_role,
            set_probe_failed,
            connection_epoch,
            2,
            "proxy".to_string(),
        ));
        assert_eq!(node_role.get_untracked(), "");
        assert!(probe_failed.get_untracked());
    }
}
