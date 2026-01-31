// crates/core/src/plugin/runtime/chat_stream.rs
//! # AI Chat Streaming Bridge
//!
//! Provides a lightweight bridge between Rhai host functions and
//! the runtime-specific streaming implementation supplied by the server.
//!
//! ## Architecture Overview
//! ```text
//! ┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │ Rhai Plugin │────▶│ ChatStreamBridge │────▶│ Server Handler  │
//! │ (ai-chat)   │     │ (this module)    │     │ (reqwest/tokio) │
//! └─────────────┘     └──────────────────┘     └─────────────────┘
//!        ▲                    │
//!        │                    ▼
//!        │            ┌──────────────────┐
//!        └────────────│ ChatStreamSink   │──▶ WebSocket Client
//!                     └──────────────────┘
//! ```
//!
//! ## Design Constraints
//! - **Runtime Agnostic**: Core crate 不依赖 tokio，保持轻量
//! - **Thread Safety**: Handler 为全局单例 (OnceLock)，Sink 为线程局部 (thread_local)
//! - **Memory Budget**: 适配 768MB VPS 环境
//!
//! ## Invariants (不变量)
//! 1. `CHAT_STREAM_HANDLER` 在进程生命周期内只能设置一次
//! 2. `ChatStreamScope` 的生命周期必须完全覆盖 Rhai 脚本执行期
//! 3. 任意时刻，每个线程最多只有一个活跃的 `ChatStreamSink`

use crate::protocol::ServerMessage;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

/// Streaming request payload passed to the handler.
#[derive(Debug, Clone)]
pub struct ChatStreamRequest {
    pub req_id: String,
    pub config: Value,
    pub history: Value,
    pub tools: Option<Value>,
}

/// Response from chat stream - can be text or tool calls
///
/// ## Variants
/// - `Text`: 普通文本响应，流式传输已完成
/// - `ToolCalls`: AI 请求调用工具，需要 tool_loop 处理
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatStreamResponse {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "tool_calls")]
    ToolCalls { calls: Vec<ToolCallInfo> },
}

/// Tool call information from AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON string
}

/// Streaming handler interface (implemented by the server runtime).
///
/// ## Implementor Requirements
/// - 实现者必须处理 SSE 流并通过 sink 发送 chunks
/// - 网络错误应转换为 `anyhow::Error` 返回
pub trait ChatStreamHandler: Send + Sync {
    fn stream(
        &self,
        request: ChatStreamRequest,
        sink: ChatStreamSink,
    ) -> Result<ChatStreamResponse>;
}

/// Sink for streaming `ChatChunk` messages back to the client.
///
/// ## Thread Safety
/// - `Clone` 是廉价的 (Arc 内部)
/// - `send_chunk` 可从任意线程调用
#[derive(Clone)]
pub struct ChatStreamSink {
    sender: Arc<dyn Fn(ServerMessage) + Send + Sync>,
}

impl ChatStreamSink {
    pub fn new<F>(sender: F) -> Self
    where
        F: Fn(ServerMessage) + Send + Sync + 'static,
    {
        Self {
            sender: Arc::new(sender),
        }
    }

    pub fn send_chunk(&self, req_id: &str, delta: Option<String>, finish_reason: Option<String>) {
        (self.sender)(ServerMessage::ChatChunk {
            req_id: req_id.to_string(),
            delta,
            finish_reason,
        });
    }
}

/// Global handler storage (process-wide singleton)
static CHAT_STREAM_HANDLER: OnceLock<Arc<dyn ChatStreamHandler>> = OnceLock::new();

// Thread-local sink for per-request routing
// Invariant: 在 `ChatStreamScope` 活跃期间，此值必须为 `Some`
thread_local! {
    static CHAT_STREAM_SINK: RefCell<Option<ChatStreamSink>> = const { RefCell::new(None) };
}

/// 设置全局 Handler (只能调用一次)
pub fn set_chat_stream_handler(handler: Arc<dyn ChatStreamHandler>) -> Result<()> {
    CHAT_STREAM_HANDLER
        .set(handler)
        .map_err(|_| anyhow!("Chat stream handler already set"))
}

/// 获取全局 Handler
pub fn chat_stream_handler() -> Option<Arc<dyn ChatStreamHandler>> {
    CHAT_STREAM_HANDLER.get().cloned()
}

/// RAII guard for thread-local sink injection.
///
/// ## Invariants
/// - 构造时将 sink 注入 thread_local
/// - 析构时恢复之前的 sink (支持嵌套，虽然当前设计不需要)
///
/// ## Usage
/// ```ignore
/// let _scope = ChatStreamScope::new(sink);
/// // sink 在此作用域内可用
/// plugin.call("chat", args);
/// // _scope drop 时自动清理
/// ```
pub struct ChatStreamScope {
    previous: Option<ChatStreamSink>,
}

impl ChatStreamScope {
    pub fn new(sink: ChatStreamSink) -> Self {
        let previous = CHAT_STREAM_SINK.with(|cell| cell.replace(Some(sink)));
        Self { previous }
    }
}

impl Drop for ChatStreamScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        CHAT_STREAM_SINK.with(|cell| {
            cell.replace(previous);
        });
    }
}

/// 获取当前线程的 Sink
pub fn current_chat_stream_sink() -> Option<ChatStreamSink> {
    CHAT_STREAM_SINK.with(|cell| cell.borrow().clone())
}
