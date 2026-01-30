// apps/cli/src/server/ai_chat/mod.rs
//! # AI Chat Streaming (Server Runtime)
//!
//! **功能**: OpenAI 兼容的流式聊天实现。
//!
//! **模块结构**:
//! - `config`: 配置结构
//! - `types`: SSE 响应数据类型
//! - `sse_parser`: SSE 消息解析与工具调用构建
//! - `stream`: 流式请求执行
//!
//! **优化**:
//! - 全局 HTTP 客户端单例 (复用 TCP 连接池)
//! - 强类型 SSE 解析 (避免 serde_json::Value)

mod config;
mod sse_parser;
mod stream;
mod types;

use anyhow::{anyhow, Result};
use config::ChatConfig;
use deve_core::plugin::runtime::chat_stream::{
    set_chat_stream_handler, ChatStreamHandler, ChatStreamRequest,
    ChatStreamResponse, ChatStreamSink,
};
use serde_json::json;
use std::sync::Arc;
use stream::execute_stream;

pub fn init_chat_stream_handler() -> Result<()> {
    let handler = Arc::new(AiChatStreamHandler);
    set_chat_stream_handler(handler)
}

struct AiChatStreamHandler;

impl ChatStreamHandler for AiChatStreamHandler {
    fn stream(
        &self,
        request: ChatStreamRequest,
        sink: ChatStreamSink,
    ) -> Result<ChatStreamResponse> {
        let config: ChatConfig = serde_json::from_value(request.config)
            .map_err(|e| anyhow!("Invalid AI config: {}", e))?;
        config.validate().map_err(|e| anyhow!("{}", e))?;

        let history = request
            .history
            .as_array()
            .ok_or_else(|| anyhow!("Chat history must be an array"))?
            .clone();

        let mut body = json!({
            "model": config.model,
            "messages": history,
            "stream": true,
            "max_tokens": config.max_tokens,
        });

        if let Some(tools) = &request.tools {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("tools".to_string(), tools.clone());
            }
        }

        let req_id = request.req_id.clone();
        let endpoint = config.endpoint();
        let headers = config.headers.clone();
        let api_key = config.api_key.clone();

        tokio::runtime::Handle::current().block_on(async move {
            execute_stream(&req_id, &endpoint, &api_key, &headers, body, &sink).await
        })
    }
}
