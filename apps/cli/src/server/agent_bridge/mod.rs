//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! Agent bridge: 将聊天请求桥接到外部 CLI。

mod policy;
mod prompt;
mod stream;

use crate::server::channel::DualChannel;
use crate::server::plugin_response::{send_plugin_invalid_message, send_plugin_request_failed};
use axum::Json;
use deve_core::protocol::ServerMessage;
use std::sync::{LazyLock, RwLock};
use tokio::sync::Semaphore;

static IN_FLIGHT: Semaphore = Semaphore::const_new(1);

static POLICY: LazyLock<RwLock<policy::AgentBridgePolicy>> = LazyLock::new(|| {
    RwLock::new(policy::AgentBridgePolicy::from_config(
        &deve_core::config::Config::load(),
    ))
});

pub fn init_from_config(config: &deve_core::config::Config) {
    if let Ok(mut policy) = POLICY.write() {
        *policy = policy::AgentBridgePolicy::from_config(config);
    }
}

pub async fn http_backend_capabilities() -> Json<policy::AgentBridgeCapabilities> {
    Json(capabilities())
}

/// Invariants:
/// 1. 子进程生命周期严格受本函数调用链控制。
/// 2. 失败路径必须同时结束 plugin 请求与 chat streaming。
pub async fn handle_agent_chat(ch: &DualChannel, req_id: String, args: Vec<serde_json::Value>) {
    let user_message = prompt::extract_user_message(&args);
    if user_message.is_empty() {
        send_plugin_invalid_message(ch, &req_id, "No user message provided");
        finish_chat(ch, &req_id);
        return;
    }

    let run_config = match run_config() {
        Ok(config) => config,
        Err(detail) => {
            tracing::warn!("Agent bridge blocked: {}", detail);
            send_plugin_request_failed(ch, &req_id, detail);
            finish_chat(ch, &req_id);
            return;
        }
    };
    let Ok(_permit) = IN_FLIGHT.try_acquire() else {
        send_plugin_request_failed(ch, &req_id, "external agent busy");
        finish_chat(ch, &req_id);
        return;
    };
    tracing::info!(
        "Agent bridge: spawning `{}` with query len={}",
        run_config.cli_path,
        user_message.len()
    );

    match stream::spawn_and_stream(
        &run_config.cli_path,
        &user_message,
        run_config.timeout_ms,
        ch,
        &req_id,
    )
    .await
    {
        Ok(()) => tracing::info!("Agent bridge: session completed for req_id={}", req_id),
        Err(err) => {
            tracing::error!("Agent bridge error: {:?}", err);
            send_plugin_request_failed(ch, &req_id, format!("Agent CLI error: {}", err));
            finish_chat(ch, &req_id);
        }
    }
}

fn finish_chat(ch: &DualChannel, req_id: &str) {
    ch.unicast(ServerMessage::ChatChunk {
        req_id: req_id.to_string(),
        delta: None,
        finish_reason: Some("stop".to_string()),
    });
}

fn capabilities() -> policy::AgentBridgeCapabilities {
    POLICY
        .read()
        .map(|policy| policy.capabilities())
        .unwrap_or(policy::AgentBridgeCapabilities {
            native_available: false,
            native_reason: Some("AI backend policy unavailable".to_string()),
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            effective_backend: "none".to_string(),
            effective_backend_reason: Some("AI backend policy unavailable".to_string()),
        })
}

fn run_config() -> Result<policy::AgentBridgeRunConfig, String> {
    POLICY
        .read()
        .map_err(|_| "external agent disabled".to_string())?
        .run_config()
}

#[cfg(test)]
mod http_tests;
