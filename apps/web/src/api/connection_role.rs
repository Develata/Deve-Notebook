//! plan_ref:
//!   - 15_release#runtime-observability
//!

use futures::FutureExt;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;

use super::connection::ConnectionLifecycle;

const NODE_ROLE_PROBE_RETRIES: usize = 3;
const NODE_ROLE_PROBE_RETRY_DELAY_MS: u32 = 150;
const NODE_ROLE_PROBE_TIMEOUT_MS: u32 = 1_500;

pub(super) async fn fetch_node_role(
    lifecycle: ConnectionLifecycle,
    ws_url: String,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) {
    fetch_node_role_for_http_base(
        lifecycle,
        http_base_from_ws_url(&ws_url),
        set_node_role,
        set_node_role_probe_failed,
        current_connection_epoch,
        probe_connection_epoch,
    )
    .await;
}

pub(super) async fn fetch_node_role_for_http_base(
    lifecycle: ConnectionLifecycle,
    http_base: String,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) {
    let url = node_role_url_for_http_base(&http_base);
    let summary = probe_node_role_summary_with_retries(&url, || lifecycle.is_active()).await;
    if !lifecycle.is_active() {
        return;
    }

    if let Some(summary) = summary {
        apply_node_role_probe_success(
            &lifecycle,
            set_node_role,
            set_node_role_probe_failed,
            current_connection_epoch,
            probe_connection_epoch,
            summary,
        );
        return;
    }
    leptos::logging::error!("Node role probe failed after retries: {}", url);
    apply_node_role_probe_failure(
        &lifecycle,
        set_node_role,
        set_node_role_probe_failed,
        current_connection_epoch,
        probe_connection_epoch,
    );
}

pub(crate) async fn probe_node_role_summary_for_http_base(http_base: String) -> Option<String> {
    let url = node_role_url_for_http_base(&http_base);
    probe_node_role_summary_with_retries(&url, || true).await
}

async fn probe_node_role_summary_with_retries(
    url: &str,
    mut should_continue: impl FnMut() -> bool,
) -> Option<String> {
    for attempt in 0..NODE_ROLE_PROBE_RETRIES {
        if !should_continue() {
            return None;
        }
        if let Some(json) = fetch_node_role_json_with_timeout(url).await {
            return Some(format_node_role_summary(&json));
        }
        if attempt + 1 < NODE_ROLE_PROBE_RETRIES {
            TimeoutFuture::new(NODE_ROLE_PROBE_RETRY_DELAY_MS).await;
        }
    }
    None
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
    lifecycle: &ConnectionLifecycle,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
    summary: String,
) -> bool {
    if lifecycle.try_get(current_connection_epoch) != Some(probe_connection_epoch) {
        return false;
    }
    lifecycle.try_set(set_node_role, summary)
        && lifecycle.try_set(set_node_role_probe_failed, false)
}

fn apply_node_role_probe_failure(
    lifecycle: &ConnectionLifecycle,
    set_node_role: WriteSignal<String>,
    set_node_role_probe_failed: WriteSignal<bool>,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) -> bool {
    if lifecycle.try_get(current_connection_epoch) != Some(probe_connection_epoch) {
        return false;
    }
    lifecycle.try_set(set_node_role, String::new())
        && lifecycle.try_set(set_node_role_probe_failed, true)
}

pub(crate) fn http_base_from_ws_url(ws_url: &str) -> String {
    let http_url = match ws_url.strip_prefix("wss://") {
        Some(rest) => format!("https://{rest}"),
        None => match ws_url.strip_prefix("ws://") {
            Some(rest) => format!("http://{rest}"),
            None => ws_url.to_string(),
        },
    };
    strip_ws_path_suffix(&http_url)
}

fn strip_ws_path_suffix(http_url: &str) -> String {
    let split_idx = http_url.find(['?', '#']).unwrap_or(http_url.len());
    let (base, suffix) = http_url.split_at(split_idx);
    match base.strip_suffix("/ws") {
        Some(base) => format!("{base}{suffix}"),
        None => http_url.to_string(),
    }
}

fn node_role_url_for_http_base(http_base: &str) -> String {
    let http_url = http_base.trim_end_matches('/');
    format!("{}/api/node/role", http_url)
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
mod tests;
