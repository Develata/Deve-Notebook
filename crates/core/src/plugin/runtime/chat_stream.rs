// crates/core/src/plugin/runtime/chat_stream.rs
//! # AI Chat Streaming Bridge
//!
//! Provides a lightweight bridge between Rhai host functions and
//! the runtime-specific streaming implementation supplied by the server.

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
    pub tools: Option<Value>, // Optional tools for function calling
}

/// Response from chat stream - can be text or tool calls
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
pub trait ChatStreamHandler: Send + Sync {
    fn stream(
        &self,
        request: ChatStreamRequest,
        sink: ChatStreamSink,
    ) -> Result<ChatStreamResponse>;
}

/// Sink for streaming `ChatChunk` messages back to the client.
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

static CHAT_STREAM_HANDLER: OnceLock<Arc<dyn ChatStreamHandler>> = OnceLock::new();

thread_local! {
    static CHAT_STREAM_SINK: RefCell<Option<ChatStreamSink>> = const { RefCell::new(None) };
}

pub fn set_chat_stream_handler(handler: Arc<dyn ChatStreamHandler>) -> Result<()> {
    CHAT_STREAM_HANDLER
        .set(handler)
        .map_err(|_| anyhow!("Chat stream handler already set"))
}

pub fn chat_stream_handler() -> Option<Arc<dyn ChatStreamHandler>> {
    CHAT_STREAM_HANDLER.get().cloned()
}

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

pub fn current_chat_stream_sink() -> Option<ChatStreamSink> {
    CHAT_STREAM_SINK.with(|cell| cell.borrow().clone())
}
