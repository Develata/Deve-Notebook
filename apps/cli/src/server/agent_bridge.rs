// apps/cli/src/server/agent_bridge.rs
//! # Agent Bridge (外部 CLI 桥接)
//!
//! 将 AI 聊天请求桥接到外部成熟的 CLI 工具 (如 zeroclaw/opencode)，
//! 通过子进程管道流式传输响应，避免重复造轮子。
//!
//! ## 架构
//! ```text
//! Frontend (WS) ──► handle_agent_chat()
//!                        │
//!                        ▼
//!                  tokio::process::Command("zeroclaw")
//!                        │ stdout pipe
//!                        ▼
//!                  DualChannel ──► ServerMessage::ChatChunk ──► WS
//! ```
//!
//! ## Invariants
//! 1. 子进程生命周期严格受 `handle_agent_chat` 管控，函数退出即回收
//! 2. 任何 spawn 失败不会 panic，只返回错误到前端

use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;

/// 默认 CLI 工具名 (可通过 `AGENT_CLI_PATH` 环境变量覆盖)
const DEFAULT_CLI: &str = "opencode";

/// 处理来自前端的 Agent 聊天请求。
///
/// 从 `args` 中提取用户消息，spawn 外部 CLI 子进程，
/// 将 stdout 逐行转为 `ChatChunk` 推送给客户端。
pub async fn handle_agent_chat(ch: &DualChannel, req_id: String, args: Vec<serde_json::Value>) {
    let user_message = extract_user_message(&args);
    if user_message.is_empty() {
        send_error(ch, &req_id, "No user message provided");
        return;
    }

    let cli_path = std::env::var("AGENT_CLI_PATH").unwrap_or_else(|_| DEFAULT_CLI.to_string());

    tracing::info!(
        "Agent bridge: spawning `{}` with query len={}",
        cli_path,
        user_message.len()
    );

    match spawn_and_stream(&cli_path, &user_message, ch, &req_id).await {
        Ok(()) => {
            tracing::info!("Agent bridge: session completed for req_id={}", req_id);
        }
        Err(e) => {
            tracing::error!("Agent bridge error: {:?}", e);
            send_error(ch, &req_id, &format!("Agent CLI error: {}", e));
            // 必须发送 finish 信号，否则前端 streaming 状态永远不会结束
            ch.unicast(ServerMessage::ChatChunk {
                req_id: req_id.clone(),
                delta: None,
                finish_reason: Some("stop".to_string()),
            });
        }
    }
}

/// 从插件调用参数中提取用户消息文本。
///
/// 支持两种格式:
/// - `args[1]` 作为纯字符串 (原 ai-chat 调用约定: req_id, message, context)
/// - `args[0]` 作为纯字符串 (简化调用)
fn extract_user_message(args: &[serde_json::Value]) -> String {
    // 原 ai-chat 约定: chat(req_id, user_message, context)
    if args.len() >= 2 {
        if let Some(s) = args[1].as_str() {
            return s.to_string();
        }
    }
    // 简化: 第一个参数即为消息
    if let Some(first) = args.first() {
        if let Some(s) = first.as_str() {
            return s.to_string();
        }
    }
    String::new()
}

/// 启动外部 CLI 并将 stdout 流式推送到前端。
async fn spawn_and_stream(
    cli_path: &str,
    query: &str,
    ch: &DualChannel,
    req_id: &str,
) -> anyhow::Result<()> {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;

    let mut child = tokio::process::Command::new(cli_path)
        .args(["run", query])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to spawn '{}': {}. Is it installed and in PATH?",
                cli_path,
                e
            )
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    let mut reader = tokio::io::BufReader::new(stdout);

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let clean = strip_ansi(&line);
                if !clean.trim().is_empty() {
                    ch.unicast(ServerMessage::ChatChunk {
                        req_id: req_id.to_string(),
                        delta: Some(clean),
                        finish_reason: None,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Agent stdout read error: {:?}", e);
                break;
            }
        }
    }

    // 发送完成信号
    ch.unicast(ServerMessage::ChatChunk {
        req_id: req_id.to_string(),
        delta: None,
        finish_reason: Some("stop".to_string()),
    });

    // 等待子进程退出，确保资源回收
    let status = child.wait().await?;
    if !status.success() {
        tracing::warn!("Agent CLI exited with status: {}", status);
    }

    Ok(())
}

/// 剥离 ANSI 转义序列 (如终端颜色码 `\x1b[31m`)。
///
/// 使用手写状态机，零依赖，O(n) 复杂度。
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // 跳过 ESC[ ... m 序列
            if chars.next() == Some('[') {
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 向客户端发送错误响应。
fn send_error(ch: &DualChannel, req_id: &str, message: &str) {
    ch.unicast(ServerMessage::PluginResponse {
        req_id: req_id.to_string(),
        result: None,
        error: Some(message.to_string()),
    });
}
