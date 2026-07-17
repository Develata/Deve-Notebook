//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 18_release#runtime-observability
//!

use futures::FutureExt;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;

use super::connection::ConnectionLifecycle;

mod payload;

#[cfg(test)]
use self::payload::format_node_role_summary;
use self::payload::node_role_url_for_http_base;
pub(crate) use self::payload::{
    NodeRoleProbeResult, WatcherHealthSnapshot, WatcherHealthStatus, http_base_from_ws_url,
};

const NODE_ROLE_PROBE_RETRIES: usize = 3;
const NODE_ROLE_PROBE_RETRY_DELAY_MS: u32 = 150;
const NODE_ROLE_PROBE_TIMEOUT_MS: u32 = 1_500;

pub(super) struct NodeRoleProbeContext {
    pub set_node_role: WriteSignal<String>,
    pub set_source_control_authority: WriteSignal<String>,
    pub set_host_file_copy_absolute_path: WriteSignal<bool>,
    pub set_host_file_reveal_in_system_explorer: WriteSignal<bool>,
    pub set_watcher_health: WriteSignal<WatcherHealthSnapshot>,
    pub set_node_role_probe_failed: WriteSignal<bool>,
    pub current_connection_epoch: ReadSignal<u64>,
    pub probe_connection_epoch: u64,
}

pub(super) async fn fetch_node_role(
    lifecycle: ConnectionLifecycle,
    ws_url: String,
    context: NodeRoleProbeContext,
) {
    fetch_node_role_for_http_base(lifecycle, http_base_from_ws_url(&ws_url), context).await;
}

pub(super) async fn fetch_node_role_for_http_base(
    lifecycle: ConnectionLifecycle,
    http_base: String,
    context: NodeRoleProbeContext,
) {
    let url = node_role_url_for_http_base(&http_base);
    let summary = probe_node_role_summary_with_retries(&url, || lifecycle.is_active()).await;
    if !lifecycle.is_active() {
        return;
    }

    if let Some(summary) = summary {
        apply_node_role_probe_success(&lifecycle, context, summary);
        return;
    }
    leptos::logging::error!("Node role probe failed after retries: {}", url);
    apply_node_role_probe_failure(&lifecycle, context);
}

pub(crate) async fn probe_node_role_for_http_base(
    http_base: String,
) -> Option<NodeRoleProbeResult> {
    let url = node_role_url_for_http_base(&http_base);
    probe_node_role_summary_with_retries(&url, || true).await
}

async fn probe_node_role_summary_with_retries(
    url: &str,
    mut should_continue: impl FnMut() -> bool,
) -> Option<NodeRoleProbeResult> {
    for attempt in 0..NODE_ROLE_PROBE_RETRIES {
        if !should_continue() {
            return None;
        }
        if let Some(json) = fetch_node_role_json_with_timeout(url).await
            && let Some(result) = NodeRoleProbeResult::from_json(&json)
        {
            return Some(result);
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
    context: NodeRoleProbeContext,
    result: NodeRoleProbeResult,
) -> bool {
    if lifecycle.try_get(context.current_connection_epoch) != Some(context.probe_connection_epoch) {
        return false;
    }
    lifecycle.try_set(context.set_node_role, result.summary)
        && lifecycle.try_set(
            context.set_source_control_authority,
            result.source_control_authority,
        )
        && lifecycle.try_set(
            context.set_host_file_copy_absolute_path,
            result.host_file_copy_absolute_path,
        )
        && lifecycle.try_set(
            context.set_host_file_reveal_in_system_explorer,
            result.host_file_reveal_in_system_explorer,
        )
        && lifecycle.try_set(context.set_watcher_health, result.watcher_health)
        && lifecycle.try_set(context.set_node_role_probe_failed, false)
}

fn apply_node_role_probe_failure(
    lifecycle: &ConnectionLifecycle,
    context: NodeRoleProbeContext,
) -> bool {
    if lifecycle.try_get(context.current_connection_epoch) != Some(context.probe_connection_epoch) {
        return false;
    }
    lifecycle.try_set(context.set_node_role, String::new())
        && lifecycle.try_set(context.set_source_control_authority, "unknown".to_string())
        && lifecycle.try_set(context.set_host_file_copy_absolute_path, false)
        && lifecycle.try_set(context.set_host_file_reveal_in_system_explorer, false)
        && lifecycle.try_set(context.set_watcher_health, WatcherHealthSnapshot::default())
        && lifecycle.try_set(context.set_node_role_probe_failed, true)
}

#[cfg(test)]
mod tests;
