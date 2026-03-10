//! Agent bridge: 将聊天请求桥接到外部 CLI。

mod prompt;
mod stream;

use crate::server::channel::DualChannel;
use crate::server::plugin_response::{send_plugin_invalid_message, send_plugin_request_failed};
use deve_core::protocol::ServerMessage;

/// 默认 CLI 工具名 (可通过 `AGENT_CLI_PATH` 环境变量覆盖)。
const DEFAULT_CLI: &str = "opencode";

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

    let cli_path = std::env::var("AGENT_CLI_PATH").unwrap_or_else(|_| DEFAULT_CLI.to_string());
    tracing::info!(
        "Agent bridge: spawning `{}` with query len={}",
        cli_path,
        user_message.len()
    );

    match stream::spawn_and_stream(&cli_path, &user_message, ch, &req_id).await {
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
